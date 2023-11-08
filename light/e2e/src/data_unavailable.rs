// Copyright 2023 ZeroDAO

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at

//     http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::{
    debug, delay_for_seconds, error, info, request, wait_for_block_confirmation,
    wait_for_finalized_success, BlobTxSatus, Result, WsClient, H256,
};
use melo_core_primitives::{
    config::{EXTENDED_SEGMENTS_PER_BLOB, SEGMENTS_PER_BLOB},
    reliability::{sample_key, sample_key_from_block},
    Position,
};
use meloxt::{commitments_to_runtime, info_msg::*, sidecar_metadata, Client, ClientSync};
use subxt::{rpc::types::Bytes, rpc_params};

// This function performs operations related to data unavailability.
pub(crate) async fn run(client: &Client, ws_client: &WsClient) -> Result<()> {
    // Log the start of the data unavailability process.
    info!("{}: Running data_unavailable", START_EXAMPLE);

    let app_id = 1;
    let bytes_len = 123;

    // Retrieve and increment the nonce for the application ID.
    let mut nonce = client.nonce(app_id).await?;
    nonce += 1;

    // Generate sidecar metadata and bytes associated with the app and nonce.
    let (sidecar_metadata, bytes) = sidecar_metadata(bytes_len, app_id, nonce);

    // Log the number of commitments generated.
    let row_count = sidecar_metadata.commitments.len();
    debug!("{}: Commitments len: {:?}", SUCCESS, row_count);

    // Convert commitments to a runtime-compatible format and collect into bytes.
    let commitments_t = sidecar_metadata.commitments.clone();
    let commitments = commitments_to_runtime(commitments_t.clone());
    let commitments_bytes =
        commitments.iter().flat_map(|c| c.inner.clone().to_vec()).collect::<Vec<_>>();
    debug!("{}: Commitments bytes: {:?}", SUCCESS, commitments_bytes);

    // Create parameters for the blob transaction and log them.
    let (hex_bytes, hex_extrinsic) = client.create_params(bytes, &sidecar_metadata.clone()).await?;
    let params = rpc_params![hex_bytes, hex_extrinsic];
    debug!("Params of das_submitBlobTx: {:?}", params.clone().build().unwrap().get());

    // Submit the blob transaction and log the result.
    let res: BlobTxSatus<H256> = client.api.rpc().request("das_submitBlobTx", params).await?;
    debug!("Data submitted: {:?}", res);
    info!("{}: Data submitted, tx_hash: {:?}", SUCCESS, res.tx_hash);

    // Handle errors in blob transaction submission.
    if let Some(err) = res.err {
        error!("{}: Failed to submit blob transaction: {:?}", ERROR, err);
        return Err(anyhow::anyhow!("Failed to submit blob transaction"))
    }

    // Wait for block confirmation after submitting the blob.
    let (at_block, at_block_hash) = wait_for_block_confirmation(client, &commitments_bytes).await?;

    // Wait for the data to be propagated across the network.
    info!("{}: Waiting for data to be propagated across the network.", HOURGLASS);
    delay_for_seconds(3);

    // Generate keys for the block and the application.
    let block_keys: Vec<_> = (row_count..row_count * 2)
        .flat_map(|y| {
            (0..EXTENDED_SEGMENTS_PER_BLOB).map(move |x| {
                sample_key_from_block(
                    at_block_hash.as_bytes(),
                    &Position { x: x as u32, y: y as u32 },
                )
            })
        })
        .collect();

    let app_keys: Vec<_> = (0..SEGMENTS_PER_BLOB)
        .flat_map(|x| {
            (0..row_count)
                .map(move |y| sample_key(app_id, nonce, &Position { x: x as u32, y: y as u32 }))
        })
        .collect();

    // Collect keys and convert them to Bytes type.
    let keys: Vec<Bytes> = block_keys
        .into_iter()
        .chain(app_keys.into_iter())
        .map(|key| Bytes::from(key))
        .collect();

    // Remove records from full nodes and light clients.
    let params = rpc_params![keys];
    let _ = client.api.rpc().request("das_removeRecords", params.clone()).await?;
    let _ = request(ws_client, "das_removeRecords", Some(params)).await?;
    info!("{}: 75% of data has been deleted", SUCCESS);

    // Finalize the block after data deletion.
    wait_for_finalized_success(client, at_block).await?;

    // Wait for the sampling to complete before fetching confidence.
    info!("{}: Wait for the sampling to complete.", HOURGLASS);
    delay_for_seconds(3);

    // Request the block confidence and handle the response.
    let params = rpc_params![at_block_hash];
    let response = request(ws_client, "das_blockConfidence", Some(params)).await?;
    let val: Option<u32> = serde_json::from_str(response.get())?;

    // Check the block confidence and log the status.
    if let Some(confidence) = val {
        if confidence < 999_900 {
            info!("{}: Block confidence is less than 99.99%: {:?}", SUCCESS, confidence);
        } else {
            info!("{}: Block confidence is greater than 99.99%: {:?}, it should be unavailable.", ERROR, confidence);
        }
    } else {
        info!("{}: Block confidence is None: {:?}", ERROR, response);
    }

    // Log the completion of the data unavailability module process.
    info!("{}: Module data_unavailable", ALL_SUCCESS);

    Ok(())
}

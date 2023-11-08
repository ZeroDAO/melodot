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
use meloxt::{commitments_to_runtime, info_msg::*, melodot, sidecar_metadata, Client, ClientSync};
use subxt::rpc_params;

// This function encapsulates the workflow for running data availability operations.
pub(crate) async fn run(client: &Client, ws_client: &WsClient) -> Result<()> {
    // Log the start of the data availability process.
    info!("{}: Running data availability", START_EXAMPLE);

    // Register an application on the blockchain.
    let register_app_tx = melodot::tx().melo_store().register_app();

    // Submit the transaction and wait for it to be finalized successfully.
    let res = client
        .api
        .tx()
        .sign_and_submit_then_watch_default(&register_app_tx, &client.signer)
        .await?
        .wait_for_finalized_success()
        .await?;

    // Log the successful creation of the application.
    info!("{} Application created, block hash: {}", SUCCESS, res.block_hash());

    // Define application ID and the length of the bytes to work with.
    let app_id = 1;
    let bytes_len = 123;

    // Retrieve the current nonce for the given application ID.
    let nonce = client.nonce(app_id).await?;

    // Generate sidecar metadata and the bytes associated with it.
    let (sidecar_metadata, bytes) = sidecar_metadata(bytes_len, app_id, nonce + 1);

    debug!("{}: Commitments len: {:?}", SUCCESS, sidecar_metadata.commitments.len());

    // Convert commitments to a format suitable for the runtime.
    let commitments_t = sidecar_metadata.commitments.clone();
    let commitments = commitments_to_runtime(commitments_t.clone());
    let commitments_bytes =
        commitments.iter().flat_map(|c| c.inner.clone().to_vec()).collect::<Vec<_>>();

    debug!("{}: Commitments bytes: {:?}", SUCCESS, commitments_bytes);

    // Create parameters required for submitting the blob transaction.
    let (hex_bytes, hex_extrinsic) = client.create_params(bytes, &sidecar_metadata.clone()).await?;
    let params = rpc_params![hex_bytes, hex_extrinsic];
    debug!("Params of das_submitBlobTx: {:?}", params.clone().build().unwrap().get());

    // Submit the blob transaction and await the response.
    let res: BlobTxSatus<H256> = client.api.rpc().request("das_submitBlobTx", params).await?;

    debug!("Data submitted: {:?}", res);

    info!("{}: Data submitted, tx_hash: {:?}", SUCCESS, res.tx_hash);
    if let Some(err) = res.err {
        error!("{}: Failed to submit blob transaction: {:?}", ERROR, err);
        return Err(anyhow::anyhow!("Failed to submit blob transaction"))
    }

    // Wait for block confirmation and then wait for it to be finalized.
    let (at_block, at_block_hash) = wait_for_block_confirmation(client, &commitments_bytes).await?;
    wait_for_finalized_success(client, at_block).await?;

    // We need to wait for the sampling to complete before checking data availability.
	info!("{} Wait for the sampling to complete.", HOURGLASS);
    delay_for_seconds(3);

    // Request the block confidence value.
    let params = rpc_params![at_block_hash];
    let response = request(ws_client, "das_blockConfidence", Some(params)).await?;

    // Deserialize the block confidence value from the response.
    let val: Option<u32> = serde_json::from_str(response.get())?;

    // Check the block confidence value and log the status.
    if let Some(confidence) = val {
        if confidence > 999_900 {
            info!("{}: Block confidence is above 99.99%: {:?}", SUCCESS, confidence);
        } else {
            info!("{}: Block confidence is below 99.99%: {:?}, data unavailability.", ERROR, confidence);
        }
    } else {
        info!("{}: Block confidence is None: {:?}", ERROR, response);
		// Log and return an error if fetching the confidence fails.
		return Err(anyhow::anyhow!("Failed to retrieve confidence"))
    }

	info!("{} : Module data_availability", ALL_SUCCESS);

    Ok(())
}

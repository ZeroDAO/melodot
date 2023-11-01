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

use futures::StreamExt;
use log::{debug, error, info};
use melo_das_primitives::crypto::{KZGCommitment as KZGCommitmentT, KZGProof as KZGProofT};
use melo_das_rpc::BlobTxSatus;
use melo_core_primitives::SidecarMetadata;
use meloxt::{
	commitments_to_runtime, info_msg::*, init_logger, melodot, proofs_to_runtime, sidecar_metadata,
	Client, ClientBuilder,
};
use primitive_types::H256;
use subxt::rpc::rpc_params;

// TODO: add to runtime
const DELAY_CHECK_THRESHOLD: u32 = 1;

#[tokio::main]
pub async fn main() {
	init_logger().unwrap();

	if let Err(err) = run().await {
		error!("{}", err);
	}
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
	info!("{} submit blob tx", START_EXAMPLE);

	let client = ClientBuilder::default().build().await?;

	let app_id = 1;
	let bytes_len = 123; // Exceeding the limit
	let (sidecar_metadata, bytes) = sidecar_metadata(bytes_len, app_id, 0);

	let commitments_t = sidecar_metadata.commitments;
	let proofs_t = sidecar_metadata.proofs;

	let commitments = commitments_to_runtime(commitments_t.clone());

	let commitments_bytes =
		commitments.iter().flat_map(|c| c.inner.clone().to_vec()).collect::<Vec<_>>();

	info!("{}: Commitments bytes: {:?}", SUCCESS, commitments_bytes);

	let (hex_bytes, hex_extrinsic) =
		create_params(&client, commitments_t, proofs_t, bytes_len, bytes, app_id)
			.await?;

	let params = rpc_params![hex_bytes, hex_extrinsic];
	debug!("Params of das_submitBlobTx: {:?}", params.clone().build().unwrap().get());

	let res: BlobTxSatus<H256> = client.api.rpc().request("das_submitBlobTx", params).await?;

	debug!("Data submited: {:?}", res);

	info!("{}: Data submited, tx_hash: {:?}", SUCCESS, res.tx_hash);

	if let Some(err) = res.err {
		error!("{} : Failed to submit blob transaction: {:?}", ERROR, err);
		return Err("Failed to submit blob transaction".into())
	}

	let mut blocks_sub = client.api.blocks().subscribe_best().await?;
	let mut max_loop = DELAY_CHECK_THRESHOLD + 1;

	while let Some(block) = blocks_sub.next().await {
		let block = block?;
		let header = block.header();
		let block_number = header.number;
		let header_commitments_bytes = header.extension.commitments_bytes.clone();

		if commitments_bytes == header_commitments_bytes {
			info!(
				"{} Data should have been verified by the validators at: {:?}",
				SUCCESS, block_number
			);
			break
		} else {
			info!("{} Data not verified yet, current block number: {:?}", HOURGLASS, block_number);
			debug!(
				"{} Data not verified yet, current header_commitments: {:?}",
				HOURGLASS, header_commitments_bytes
			);
		}

		if max_loop == 0 {
			error!("{} Data not verified after {} blocks", ERROR, DELAY_CHECK_THRESHOLD);
			return Err("Data not verified after {} blocks".into())
		}

		max_loop -= 1;
	}

	info!("{} : Submit blob tx", ALL_SUCCESS);

	Ok(())
}

async fn create_params(
	client: &Client,
	metadata: &SidecarMetadata,
	bytes: Vec<u8>,
) -> Result<(String, String), Box<dyn std::error::Error>> {
	let commitments = commitments_to_runtime(commitments);
	let proofs = proofs_to_runtime(proofs);
	let submit_data_tx =
		melodot::tx()
			.melo_store()
			.submit_data(metadata);

	let extrinsic = client
		.api
		.tx()
		.create_signed(&submit_data_tx, &client.signer, Default::default())
		.await?;

	fn to_hex_string(bytes: &[u8]) -> String {
		format!("0x{}", hex::encode(bytes))
	}

	let hex_bytes = to_hex_string(&bytes);
	let hex_extrinsic = to_hex_string(&extrinsic.encoded());

	Ok((hex_bytes, hex_extrinsic))
}

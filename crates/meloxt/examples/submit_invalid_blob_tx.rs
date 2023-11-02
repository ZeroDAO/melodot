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

use log::{error, info};
use melo_core_primitives::SidecarMetadata;
use melo_das_primitives::crypto::{KZGCommitment as KZGCommitmentT, KZGProof as KZGProofT};
use melo_das_rpc::BlobTxSatus;
use meloxt::{
	commitments_to_runtime, info_msg::*, init_logger, melodot, sidecar_metadata,
	sidecar_metadata_to_runtime, wait_for_block, Client, ClientBuilder,
};
use primitive_types::H256;
use subxt::rpc::{rpc_params, RpcParams};

#[tokio::main]
pub async fn main() {
	init_logger().unwrap();

	if let Err(err) = run().await {
		error!("{}", err);
	}
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
	info!("{} submit invalid blob tx", START_EXAMPLE);

	let client = ClientBuilder::default().build().await?;

	let app_id = 1;
	let bytes_len = 123;
	let nonce = 0;
	let (metadata, bytes) = sidecar_metadata(bytes_len, app_id, nonce);
	let commitments_t = metadata.commitments.clone();

	let commitments = commitments_to_runtime(commitments_t.clone());

	let commitments_bytes =
		commitments.iter().flat_map(|c| c.inner.clone().to_vec()).collect::<Vec<_>>();

	info!("{}: Commitments bytes: {:?}", SUCCESS, commitments_bytes);

	// Invalid blob
	// submit_invalid_blob(&client, bytes.clone(), metadata.clone()).await?;

	// Invalid extrinsic
	submit_invalid_extrinsic(&client, bytes.clone(), metadata.clone()).await?;

	// Invalid bytes_len
	submit_invalid_bytes_len(&client, bytes.clone(), metadata.clone()).await?;

	// Invalid commitments
	submit_invalid_commitments(&client, bytes.clone(), metadata.clone()).await?;

	// Invalid proofs
	submit_invalid_proofs(&client, bytes.clone(), metadata.clone()).await?;

	info!("{} : Submit invalid blob tx", ALL_SUCCESS);

	Ok(())
}

// // TODO
// async fn submit_invalid_blob(
// 	client: &Client,
// 	bytes: Vec<u8>,
// 	metadata: SidecarMetadata,
// ) -> Result<(), Box<dyn std::error::Error>> {
// 	let bytes = vec![0; bytes.len()];

// 	let (hex_bytes, hex_extrinsic) = create_params(&client, bytes, &metadata).await?;

// 	let params = rpc_params![hex_bytes, hex_extrinsic];

// 	rpc_err_handler(client, "10005".to_string(), "InvalidBlob".to_string(), &params).await
// }

async fn submit_invalid_extrinsic(
	client: &Client,
	bytes: Vec<u8>,
	metadata: SidecarMetadata,
) -> Result<(), Box<dyn std::error::Error>> {
	let (hex_bytes, _) = create_params(&client, bytes, &metadata).await?;

	let hex_extrinsic = "0x000111122223334444455556666".to_string();
	let params = rpc_params![hex_bytes, hex_extrinsic];

	rpc_err_handler(client, "10002".to_string(), "InvalidExtrinsic".to_string(), &params).await
}

async fn submit_invalid_bytes_len(
	client: &Client,
	bytes: Vec<u8>,
	metadata: SidecarMetadata,
) -> Result<(), Box<dyn std::error::Error>> {
	let bytes_len = metadata.bytes_len - 1;
	let mut metadata = metadata;
	metadata.bytes_len = bytes_len;

	let (hex_bytes, hex_extrinsic) = create_params(&client, bytes, &metadata).await?;

	let params = rpc_params![hex_bytes, hex_extrinsic];

	rpc_err_handler(client, "10005".to_string(), "InvalidBytesLen".to_string(), &params).await
}

async fn submit_invalid_commitments(
	client: &Client,
	bytes: Vec<u8>,
	metadata: SidecarMetadata,
) -> Result<(), Box<dyn std::error::Error>> {
	let commitment_invalid = KZGCommitmentT::rand();
	let commitments_invalids = metadata
		.commitments
		.iter()
		.map(|_| commitment_invalid.clone())
		.collect::<Vec<_>>();

	let mut metadata = metadata;
	metadata.commitments = commitments_invalids;

	let (hex_bytes, hex_extrinsic) = create_params(&client, bytes, &metadata).await?;

	let params = rpc_params![hex_bytes, hex_extrinsic];

	let res: BlobTxSatus<H256> = client.api.rpc().request("das_submitBlobTx", params).await?;

	if let Some(err) = res.err {
		info!("{}: Submit invalid commitments, tx failed: {:?}", SUCCESS, err);
	} else {
		info!("{}: Submit invalid commitments, but tx success", ERROR);
	}

	wait_for_block(&client).await?;

	Ok(())
}

async fn submit_invalid_proofs(
	client: &Client,
	bytes: Vec<u8>,
	metadata: SidecarMetadata,
) -> Result<(), Box<dyn std::error::Error>> {
	let proof_invalid = KZGProofT::rand();
	let proofs_invalids = metadata.proofs.iter().map(|_| proof_invalid.clone()).collect::<Vec<_>>();

	let mut metadata = metadata;
	metadata.proofs = proofs_invalids;

	let (hex_bytes, hex_extrinsic) = create_params(&client, bytes, &metadata).await?;

	let params = rpc_params![hex_bytes, hex_extrinsic];

	let res: BlobTxSatus<H256> = client.api.rpc().request("das_submitBlobTx", params).await?;

	if let Some(err) = res.err {
		info!("{}: Submit invalid proofs, tx failed: {:?}", SUCCESS, err);
	} else {
		info!("{}: Submit invalid proofs, but tx success", ERROR);
	}

	wait_for_block(&client).await?;

	Ok(())
}

async fn create_params(
	client: &Client,
	bytes: Vec<u8>,
	metadata: &SidecarMetadata,
) -> Result<(String, String), Box<dyn std::error::Error>> {
	let submit_data_tx = melodot::tx()
		.melo_store()
		.submit_data(sidecar_metadata_to_runtime(&metadata.clone()));

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

async fn rpc_err_handler(
	client: &Client,
	code: String,
	case: String,
	params: &RpcParams,
) -> Result<(), Box<dyn std::error::Error>> {
	let res: Result<BlobTxSatus<H256>, subxt::Error> =
		client.api.rpc().request("das_submitBlobTx", params.to_owned()).await;

	if res.is_err() {
		let err = res.unwrap_err().to_string();

		if err.contains(&code) {
			info!("{}: Submit {}, tx failed with code: {}", SUCCESS, case, code);
		} else {
			info!(
				"{}: Submit {}, tx failed, but the code does not match. res: {}",
				ERROR, case, err
			);
			return Err("Failed to submit blob transaction".into())
		}
	} else {
		info!("{}: Submit {}, but tx success", ERROR, case);
	}

	Ok(())
}

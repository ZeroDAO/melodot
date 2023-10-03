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
use melo_das_primitives::crypto::{KZGCommitment as KZGCommitmentT, KZGProof as KZGProofT};
use melo_das_rpc::BlobTxSatus;
use meloxt::info_msg::*;
use meloxt::Client;
use meloxt::{commitments_to_runtime, wait_for_block, init_logger, proofs_to_runtime, sidecar_metadata};
use meloxt::{melodot, ClientBuilder};
use primitive_types::H256;
use subxt::rpc::rpc_params;
use subxt::rpc::RpcParams;

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
	let bytes_len = 123; // Exceeding the limit
	let (commitments_t, proofs_t, data_hash, bytes) = sidecar_metadata(bytes_len);

	let commitments = commitments_to_runtime(commitments_t.clone());

	let commitments_bytes =
		commitments.iter().flat_map(|c| c.inner.clone().to_vec()).collect::<Vec<_>>();

	info!("{}: Commitments bytes: {:?}", SUCCESS, commitments_bytes);

	// Invalid blob
	submit_invalid_blob(
		&client,
		commitments_t.clone(),
		proofs_t.clone(),
		data_hash,
		bytes_len,
		bytes.clone(),
		app_id,
	)
	.await?;

	// Invalid extrinsic
	submit_invalid_extrinsic(
		&client,
		commitments_t.clone(),
		proofs_t.clone(),
		data_hash,
		bytes_len,
		bytes.clone(),
		app_id,
	)
	.await?;

	// Invalid data_hash
	submit_invalid_data_hash(
		&client,
		commitments_t.clone(),
		proofs_t.clone(),
		data_hash,
		bytes_len,
		bytes.clone(),
		app_id,
	)
	.await?;

	// Invalid bytes_len
	submit_invalid_bytes_len(
		&client,
		commitments_t.clone(),
		proofs_t.clone(),
		data_hash,
		bytes_len,
		bytes.clone(),
		app_id,
	)
	.await?;

	// Invalid commitments
	submit_invalid_commitments(
		&client,
		commitments_t.clone(),
		proofs_t.clone(),
		data_hash,
		bytes_len,
		bytes.clone(),
		app_id,
	)
	.await?;

	// Invalid proofs
	submit_invalid_proofs(
		&client,
		commitments_t.clone(),
		proofs_t.clone(),
		data_hash,
		bytes_len,
		bytes.clone(),
		app_id,
	)
	.await?;

	info!("{} : Submit invalid blob tx", ALL_SUCCESS);

	Ok(())
}

async fn submit_invalid_blob(
	client: &Client,
	commitments: Vec<KZGCommitmentT>,
	proofs: Vec<KZGProofT>,
	data_hash: H256,
	bytes_len: u32,
	bytes: Vec<u8>,
	app_id: u32,
) -> Result<(), Box<dyn std::error::Error>> {
	let bytes = vec![0; bytes.len()];

	let (hex_bytes, hex_extrinsic) =
		create_params(&client, commitments, proofs, data_hash, bytes_len, bytes, app_id).await?;

	let params = rpc_params![hex_bytes, hex_extrinsic];

	rpc_err_handler(client, "10005".to_string(), "InvalidBlob".to_string(), &params).await
}

async fn submit_invalid_extrinsic(
	client: &Client,
	commitments: Vec<KZGCommitmentT>,
	proofs: Vec<KZGProofT>,
	data_hash: H256,
	bytes_len: u32,
	bytes: Vec<u8>,
	app_id: u32,
) -> Result<(), Box<dyn std::error::Error>> {
	let (hex_bytes, _) =
		create_params(&client, commitments, proofs, data_hash, bytes_len, bytes, app_id).await?;
	
	let hex_extrinsic = "0x000111122223334444455556666".to_string();
	let params = rpc_params![hex_bytes, hex_extrinsic];

	rpc_err_handler(client, "10002".to_string(), "InvalidExtrinsic".to_string(), &params).await
}

async fn submit_invalid_bytes_len(
	client: &Client,
	commitments: Vec<KZGCommitmentT>,
	proofs: Vec<KZGProofT>,
	data_hash: H256,
	bytes_len: u32,
	bytes: Vec<u8>,
	app_id: u32,
) -> Result<(), Box<dyn std::error::Error>> {
	let bytes_len = bytes_len - 1;
	let (hex_bytes, hex_extrinsic) =
		create_params(&client, commitments, proofs, data_hash, bytes_len, bytes, app_id).await?;

	let params = rpc_params![hex_bytes, hex_extrinsic];

	rpc_err_handler(client, "10005".to_string(), "InvalidBytesLen".to_string(), &params).await
}

async fn submit_invalid_commitments(
	client: &Client,
	commitments: Vec<KZGCommitmentT>,
	proofs: Vec<KZGProofT>,
	data_hash: H256,
	bytes_len: u32,
	bytes: Vec<u8>,
	app_id: u32,
) -> Result<(), Box<dyn std::error::Error>> {
	let commitment_invalid = KZGCommitmentT::rand();
	let commitments_invalid =
		commitments.iter().map(|_| commitment_invalid.clone()).collect::<Vec<_>>();

	let (hex_bytes, hex_extrinsic) =
		create_params(&client, commitments_invalid, proofs, data_hash, bytes_len, bytes, app_id)
			.await?;

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

async fn submit_invalid_data_hash(
	client: &Client,
	commitments: Vec<KZGCommitmentT>,
	proofs: Vec<KZGProofT>,
	_: H256,
	bytes_len: u32,
	bytes: Vec<u8>,
	app_id: u32,
) -> Result<(), Box<dyn std::error::Error>> {
	let data_hash_invalid = H256::random();
	let (hex_bytes, hex_extrinsic) =
		create_params(&client, commitments, proofs, data_hash_invalid, bytes_len, bytes, app_id)
			.await?;

	let params = rpc_params![hex_bytes, hex_extrinsic];

	rpc_err_handler(client, "10005".to_string(), "InvalidDataHash".to_string(), &params).await
}

async fn submit_invalid_proofs(
	client: &Client,
	commitments: Vec<KZGCommitmentT>,
	proofs: Vec<KZGProofT>,
	data_hash: H256,
	bytes_len: u32,
	bytes: Vec<u8>,
	app_id: u32,
) -> Result<(), Box<dyn std::error::Error>> {
	let proof_invalid = KZGProofT::rand();
	let proofs_invalid = proofs.iter().map(|_| proof_invalid.clone()).collect::<Vec<_>>();

	let (hex_bytes, hex_extrinsic) =
		create_params(&client, commitments, proofs_invalid, data_hash, bytes_len, bytes, app_id)
			.await?;

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
	commitments: Vec<KZGCommitmentT>,
	proofs: Vec<KZGProofT>,
	data_hash: H256,
	bytes_len: u32,
	bytes: Vec<u8>,
	app_id: u32,
) -> Result<(String, String), Box<dyn std::error::Error>> {
	let commitments = commitments_to_runtime(commitments);
	let proofs = proofs_to_runtime(proofs);
	let submit_data_tx =
		melodot::tx()
			.melo_store()
			.submit_data(app_id, bytes_len, data_hash, commitments, proofs);

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
			info!("{}: Submit {}, tx failed, but the code does not match. res: {}", ERROR, case, err);
			return Err("Failed to submit blob transaction".into());
		}
	} else {
		info!("{}: Submit {}, but tx success", ERROR, case);
	}

	Ok(())
}

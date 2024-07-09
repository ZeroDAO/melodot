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
pub use anyhow::{Error, Result};
pub use jsonrpsee::ws_client::WsClient;
use jsonrpsee::{
	core::{client::ClientT, traits::ToRpcParams},
	ws_client::WsClientBuilder,
};
pub use log::{debug, error, info};
use melo_das_rpc::BlobTxSatus;
use meloxt::{info_msg::*, init_logger, Client, ClientBuilder};
use serde_json::value::RawValue;
use std::{thread, time::Duration};
use subxt::{error::RpcError, rpc::RpcFuture};
pub use subxt::{
	ext::codec::Encode,
	rpc::{RpcClient, RpcParams},
	rpc_params,
	utils::H256,
};
use tokio_stream::StreamExt;

mod data_availability;
mod data_unavailable;

pub const DEFAULT_RPC_LISTEN_ADDR: &str = "127.0.0.1:4177";

struct Params(Option<Box<RawValue>>);

impl ToRpcParams for Params {
	fn to_rpc_params(self) -> Result<Option<Box<RawValue>>, serde_json::Error> {
		Ok(self.0)
	}
}

#[tokio::main]
pub async fn main() {
	init_logger().unwrap();

	if let Err(err) = run().await {
		error!("{}", err);
	}
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
	info!("🚀 Melodot Light Client e2e starting up");

	let client = ClientBuilder::default().build().await?;

	let url = format!("ws://{}", DEFAULT_RPC_LISTEN_ADDR);
	let ws_client: jsonrpsee::ws_client::WsClient = WsClientBuilder::default().build(&url).await?;

	data_availability::run(&client, &ws_client).await?;

	data_unavailable::run(&client, &ws_client).await?;

	Ok(())
}

pub async fn wait_for_block_confirmation(
	client: &Client,
	commitments_bytes: &[u8],
) -> Result<(u32, H256)> {
	const DELAY_CHECK_THRESHOLD: u32 = 1u32;

	let mut blocks_sub = client.api.blocks().subscribe_best().await?;
	let mut max_loop = DELAY_CHECK_THRESHOLD + 1;

	let mut at_block = 0;
	let mut at_block_hash = H256::zero();

	while let Some(block) = blocks_sub.next().await {
		let block = block?;
		let header = block.header();
		let block_number = header.number;
		let header_commitments_bytes = header.extension.commitments_bytes.clone();

		if commitments_bytes == header_commitments_bytes {
			at_block = block_number;
			at_block_hash = block.hash();
			info!(
				"{} Data should have been verified by the validators at: {:?}",
				SUCCESS, block_number
			);
			break;
		} else {
			info!("{} Data not verified yet, current block number: {:?}", HOURGLASS, block_number);
			debug!(
				"{} Data not verified yet, current header_commitments: {:?}",
				HOURGLASS, header_commitments_bytes
			);
		}

		if max_loop == 0 {
			error!("{} Data not verified after {} blocks", ERROR, DELAY_CHECK_THRESHOLD);
			return Err(anyhow::anyhow!("Data not verified"));
		}

		max_loop -= 1;
	}
	Ok((at_block, at_block_hash))
}

pub async fn wait_for_finalized_success(client: &Client, at_block: u32) -> Result<()> {
	let mut blocks_sub = client.api.blocks().subscribe_finalized().await?;

	let mut max_finalized_loop = 5;

	while let Some(block) = blocks_sub.next().await {
		let block = block?;
		let header = block.header();
		let block_number = header.number;

		if block_number == at_block {
			info!("{} Data finalized at block: {:?}", SUCCESS, block_number);
			break;
		} else {
			info!("{} Data not finalized yet, current block number: {:?}", HOURGLASS, block_number);
		}

		if max_finalized_loop == 0 {
			error!("{} Data not finalized after {} blocks", ERROR, max_finalized_loop);
			return Err(anyhow::anyhow!("Data not finalized"));
		}

		max_finalized_loop -= 1;
	}
	Ok(())
}

pub fn delay_for_seconds(seconds: u64) {
	let duration = Duration::from_secs(seconds);
	thread::sleep(duration);
}

pub fn request<'a>(
	ws_client: &'a WsClient,
	method: &'a str,
	params: Option<RpcParams>,
) -> RpcFuture<'a, Box<RawValue>> {
	let params = params.unwrap_or_else(|| rpc_params![]);
	Box::pin(async move {
		let res = ws_client
			.request(method, Params(params.build()))
			.await
			.map_err(|e| RpcError::ClientError(Box::new(e)))?;
		Ok(res)
	})
}

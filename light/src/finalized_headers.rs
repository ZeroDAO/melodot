// Copyright 2023 ZeroDAO
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::time::Instant;

use anyhow::{anyhow, Context, Result};
use meloxt::{MeloConfig, MelodotHeader as Header};
use subxt::OnlineClient;
use tokio::sync::mpsc::Sender;
use tokio_stream::StreamExt;
use tracing::{error, info};

use melo_core_primitives::traits::HeaderWithCommitment;
use melo_das_db::{sqlite::SqliteDasDb, traits::DasKv};
// use melo_das_network::{DasNetworkServiceWrapper, DasNetworkOperations}; // 假设这是正确的导入
use melo_daser::{DasNetworkOperations, DasNetworkServiceWrapper, Sampling, SamplingClient};

pub async fn finalized_headers<H: HeaderWithCommitment + Sync>(
	rpc_client: OnlineClient<MeloConfig>,
	message_tx: Sender<(Header, Instant)>,
	error_sender: Sender<anyhow::Error>,
	network: DasNetworkServiceWrapper, // 更新类型为 DasNetworkServiceWrapper
	database: SqliteDasDb,
) {
	let mut client: SamplingClient<H, SqliteDasDb, DasNetworkServiceWrapper> =
		SamplingClient::new(network, database);
	let mut new_heads_sub = match rpc_client.blocks().subscribe_finalized().await {
		Ok(subscription) => subscription,
		Err(e) => {
			error!("Failed to subscribe to finalized blocks: {:?}", e);
			return
		},
	};

	while let Some(message) = new_heads_sub.next().await {
		let received_at = Instant::now();
		if let Ok(block) = message {
			let header = block.header().clone();
			info!(header.number, "Received finalized block header");
			let message = (header.clone(), received_at);
			if let Err(error) = message_tx.send(message).await.context("Send failed") {
				error!("Fail to process finalized block header: {error}");
			}

			match client.sample_block::<Header>(&header.into()).await {
				Ok(_) => {}, // 如果成功，这里可以添加相应的处理逻辑
				Err(e) => {
					error!("Sampling error: {:?}", e);
				},
			}
		}
	}

	if let Err(error) =
		error_sender.send(anyhow!("Finalized blocks subscription disconnected")).await
	{
		error!("Cannot send error to error channel: {error}");
	}
}

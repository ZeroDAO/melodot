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

use anyhow::{anyhow, Context};
use futures::lock::Mutex;
use log::{debug, error, info};
use melo_das_network::Arc;
use meloxt::{MeloConfig, MelodotHeader as Header};
use subxt::OnlineClient;
use tokio::sync::mpsc::Sender;
use tokio_stream::StreamExt;

use melo_core_primitives::traits::HeaderWithCommitment;
use melo_das_db::sqlite::SqliteDasDb;
use melo_daser::{DasNetworkServiceWrapper, Sampling, SamplingClient};

/// Subscribes to finalized block headers and processes them by sending them to a message channel and sampling them.
///
/// # Arguments
///
/// * `rpc_client` - An OnlineClient that is used to subscribe to finalized block headers.
/// * `message_tx` - A Sender that is used to send the received block headers to a message channel.
/// * `error_sender` - A Sender that is used to send errors to an error channel.
/// * `network` - A DasNetworkServiceWrapper that is used to sample the received block headers.
/// * `database` - An Arc<Mutex<SqliteDasDb>> that is used to store the received block headers.
pub async fn finalized_headers<H: HeaderWithCommitment + Sync>(
	rpc_client: OnlineClient<MeloConfig>,
	message_tx: Sender<(Header, Instant)>,
	error_sender: Sender<anyhow::Error>,
	network: DasNetworkServiceWrapper,
	database: Arc<Mutex<SqliteDasDb>>,
) {
	let client: SamplingClient<H, SqliteDasDb, DasNetworkServiceWrapper> =
		SamplingClient::new(network, database);
	let mut new_heads_sub = match rpc_client.blocks().subscribe_finalized().await {
		Ok(subscription) => {
			info!("🌐 Subscribed to finalized block headers");
			subscription
		},
		Err(e) => {
			error!("⚠️ Failed to subscribe to finalized blocks: {:?}", e);
			return
		},
	};

	while let Some(message) = new_heads_sub.next().await {
		let received_at = Instant::now();
		if let Ok(block) = message {
			let header = block.header().clone();

			let block_number = header.number;

			info!("✅ Received finalized block header #{}", block_number.clone());

			let message = (header.clone(), received_at);
			if let Err(error) = message_tx.send(message).await.context("Send failed") {
				error!("❌ Fail to process finalized block header: {error}");
			}

			match client.sample_block::<Header>(&header).await {
				Ok(_) => debug!("🔍 Sampled block header #{}", block_number),
				Err(e) => {
					error!("⚠️ Sampling error: {:?}", e);
				},
			}
		} else if let Err(e) = message {
			error!("❗ Error receiving finalized header message: {:?}", e);
		}
	}

	if let Err(error) =
		error_sender.send(anyhow!("Finalized blocks subscription disconnected")).await
	{
		error!("🚫 Cannot send error to error channel: {error}");
	}
}

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
use meloxt::{MelodotHeader, MeloConfig};
use subxt::OnlineClient;
use tokio::sync::mpsc::Sender;
use tokio_stream::StreamExt;
use tracing::{error, info};

use daser::SamplingClient;
use melo_core_primitives::{traits::DasKv, Header, ExtendedHeader};
use melo_das_network::service::Service as DasNetworkService;

pub async fn finalized_headers(
    rpc_client: OnlineClient<MeloConfig>,
    message_tx: Sender<(Header, Instant)>,
    error_sender: Sender<anyhow::Error>,
    network: DasNetworkService,
    database: Box<dyn DasKv>,
) {
    let mut client = SamplingClient::new(network, database);

    async fn subscribe_and_process(
        rpc_client: OnlineClient<MeloConfig>,
        message_tx: Sender<(Header, Instant)>,
        client: &mut SamplingClient,
    ) -> Result<()> {
        let mut new_heads_sub = rpc_client.blocks().subscribe_finalized().await?;

        while let Some(message) = new_heads_sub.next().await {
            let received_at = Instant::now();
            if let Ok(block) = message {
                let header = block.header().clone();
                info!(header.number, "Received finalized block header");
                let message = (header.clone(), received_at);
                if let Err(error) = message_tx.send(message).await.context("Send failed") {
                    error!("Fail to process finalized block header: {error}");
                }

                // Sample the finalized header with the SamplingClient
                if let Err(e) = client.sample_block(&header).await {
                    error!("Sampling error: {:?}", e);
                }
            }
        }
        Err(anyhow!("Finalized blocks subscription disconnected"))
    }

    if let Err(error) = subscribe_and_process(rpc_client, message_tx, &mut client).await {
        error!("{error}");
        if let Err(error) = error_sender.send(error).await {
            error!("Cannot send error to error channel: {error}");
        }
    }
}

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

use meloxt::MeloConfig;
use std::time::Instant;
use subxt::OnlineClient;
use tokio::sync::mpsc;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

use melo_das_db::{DasKv, Sqlite::SqliteDasDb};
use melo_das_network::{Service as DasNetworkService, DasNetworkDiscovery, DasNetworkConfig};
// use melo_core_primitives::{config};

mod finalized_headers;
mod config;

pub async fn run(config: &config::Config) -> anyhow::Result<()> {
	let rpc_url = config.rpc_url.clone();
	let subscriber = FmtSubscriber::builder().with_max_level(Level::DEBUG).finish();

	let database = SqliteDasDb::default();

	tracing::subscriber::set_global_default(subscriber)?;

	let rpc_client = OnlineClient::<MeloConfig>::connect(rpc_url).await?;

	let mut metric_registry = Registry::default();
	let libp2p_metrics = LibP2PMetrics::new(&mut metric_registry);

	let (network_worker, network_service) = melo_das_network::default(libp2p_metrics);

	if let Err(e) = network_service.init() {
		tracing::error!("Failed to initiate network discovery: {:?}", e);
		return Err(e);
	}

	// Start the network worker
	tokio::spawn(network_worker.run(&config.network_config));

	let (message_tx, _message_rx) = mpsc::channel(100);
	let (error_tx, mut error_rx) = mpsc::channel(10);

	// Start listening for finalized headers
	tokio::spawn(finalized_headers(
		rpc_client.clone(),
		message_tx,
		error_tx,
		network_service,
		database,
	));

	// Handling errors for demonstration, in a real-world application you may want a more
	// sophisticated error handling mechanism
	while let Some(error) = error_rx.recv().await {
		tracing::error!("Error in finalized headers stream: {:?}", error);
	}

	Ok(())
}

pub fn main() {
	let config = parse_args();

	tokio::runtime::Builder::new_multi_thread()
		.worker_threads(4)
		.enable_all()
		.build()
		.unwrap()
		.block_on(run(config))
		.unwrap();
}

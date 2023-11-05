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

use cli::parse_args;
use futures::lock::Mutex;
use log::{error, info};
use melo_das_primitives::KZG;
use melo_daser::DasNetworkServiceWrapper;
use meloxt::{ClientBuilder, MelodotHeader};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

use melo_das_db::sqlite::SqliteDasDb;

mod cli;
mod finalized_headers;
mod rpc;

use finalized_headers::finalized_headers;

pub async fn run(config: &cli::Config) -> anyhow::Result<()> {
	info!("🚀 Melodot Light Client starting up");

	let rpc_url = config.rpc_url.clone();
	let subscriber = FmtSubscriber::builder().with_max_level(Level::DEBUG).finish();

	let database = Arc::new(Mutex::new(SqliteDasDb::default()));
	let full_deps = rpc::FullDeps { db: database.clone() };
	let addr = rpc::run_server(&full_deps, &config.rpc_listen_addr).await?;

	info!("👂 RPC server started at: {}", addr);
	tracing::subscriber::set_global_default(subscriber)?;
	let rpc_client = match ClientBuilder::default().set_url(&rpc_url).build().await {
		Ok(client) => client,
		Err(e) => {
			error!("❌ Failed to build RPC client: {:?}", e);
			return Err(e)
		},
	};
	let (network_service, network_worker) = melo_das_network::default()?;
	let network_service_wrapper =
		DasNetworkServiceWrapper::new(network_service.into(), KZG::default_embedded().into());
	tokio::spawn(network_worker.run());

	let (message_tx, _message_rx) = mpsc::channel(100);
	let (error_tx, mut error_rx) = mpsc::channel(10);
	tokio::spawn(finalized_headers::<MelodotHeader>(
		rpc_client.api,
		message_tx,
		error_tx,
		network_service_wrapper,
		database,
	));

	while let Some(error) = error_rx.recv().await {
		error!("⚠️ Error in finalized headers stream: {:?}", error);
	}

	Ok(())
}

pub fn main() {
    let config = parse_args();

    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .expect("Failed to build runtime")
        .block_on(run(&config))
        .unwrap_or_else(|e| error!("Fatal error: {}", e));
}

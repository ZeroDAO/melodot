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

use log::{debug, error, info};
use meloxt::init_logger;
use meloxt::{melodot, ClientBuilder};
use meloxt::info_msg::*;

#[tokio::main]
pub async fn main() {
	init_logger().unwrap();

	if let Err(err) = run().await {
		error!("{}", err);
	}
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
	info!("{} register app ", START_EXAMPLE);
	let client = ClientBuilder::default().build().await?;

	let register_app_tx = melodot::tx().melo_store().register_app();

	let res = client
		.api
		.tx()
		.sign_and_submit_then_watch_default(&register_app_tx, &client.signer)
		.await?
		.wait_for_finalized_success()
		.await?;

    info!("{} Application created, block hash: {}", SUCCESS, res.block_hash());

    debug!("Application created, events: {:?}", res.all_events_in_block());

	info!("{} : Register app", ALL_SUCCESS);

	Ok(())
}

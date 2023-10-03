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

use crate::{warn, Arc, Backend, OffchainDb};
use futures::StreamExt;
use melo_core_primitives::{traits::Extractor, Encode};
use sc_network::NetworkDHTProvider;
use sc_transaction_pool_api::{InPoolTransaction, TransactionPool};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::traits::Block as BlockT;

// Define a constant for logging with a target string
const LOG_TARGET: &str = "tx_pool_listener";

use crate::{sidecar_kademlia_key, NetworkProvider, Sidecar, SidecarMetadata};

/// Parameters required for the transaction pool listener.
#[derive(Clone)]
pub struct TPListenerParams<Client, Network, TP, BE> {
	pub client: Arc<Client>,
	pub network: Arc<Network>,
	pub transaction_pool: Arc<TP>,
	pub backend: Arc<BE>,
}

/// Main function responsible for starting the transaction pool listener.
/// It monitors the transaction pool for incoming transactions and processes them accordingly.
pub async fn start_tx_pool_listener<Client, Network, TP, B, BE>(
	TPListenerParams { client, network, transaction_pool, backend }: TPListenerParams<
		Client,
		Network,
		TP,
		BE,
	>,
) where
	Network: NetworkProvider + 'static,
	TP: TransactionPool<Block = B> + 'static,
	B: BlockT + Send + Sync + 'static,
	Client: HeaderBackend<B> + ProvideRuntimeApi<B>,
	Client::Api: Extractor<B>,
	BE: Backend<B>,
{
	// Log the start of the transaction pool listener
	tracing::info!(
		target: LOG_TARGET,
		"Starting transaction pool listener.",
	);

	// Initialize the off-chain database using the backend's off-chain storage.
	// If unavailable, log a warning and return without starting the listener.
	let mut offchain_db = match backend.offchain_storage() {
		Some(offchain_storage) => OffchainDb::new(offchain_storage),
		None => {
			warn!(
				target: LOG_TARGET,
				"Can't spawn a transaction pool listener for a node without offchain storage."
			);
			return;
		},
	};

	// Get the stream of import notifications from the transaction pool
	let mut import_notification_stream = transaction_pool.import_notification_stream();

	// Process each import notification as they arrive in the stream
	while let Some(notification) = import_notification_stream.next().await {
		if let Some(transaction) = transaction_pool.ready_transaction(&notification) {
			// Encode the transaction data for processing
			let encoded = transaction.data().encode();
			let at = client.info().best_hash;

			// Extract relevant information from the encoded transaction data
			match client.runtime_api().extract(at, &encoded) {
				Ok(Some(data)) => {
					for (data_hash, bytes_len, commitments, proofs) in data {
						tracing::debug!(
							target: LOG_TARGET,
							"New blob transaction found. Hash: {:?}", data_hash,
						);

						let metadata = SidecarMetadata {
							data_len: bytes_len,
							blobs_hash: data_hash,
							commitments,
							proofs,
						};

						let fetch_value_from_network = |sidecar: &Sidecar| {
							network.get_value(&sidecar_kademlia_key(sidecar));
						};
					
						match Sidecar::from_local_outside::<B, BE>(&metadata.id(), &mut offchain_db) {
							Some(sidecar) if sidecar.status.is_none() => {
								fetch_value_from_network(&sidecar);
							},
							None => {
								let sidecar = Sidecar {
									blobs: None,
									metadata: metadata.clone(),
									status: None,
								};
								sidecar.save_to_local_outside::<B, BE>(&mut offchain_db);
								fetch_value_from_network(&sidecar);
							},
							_ => {},
						}
					}
				},
				Ok(None) => tracing::debug!(
					target: LOG_TARGET,
					"Decoding of extrinsic failed. Transaction: {:?}",
					transaction.hash(),
				),
				Err(err) => tracing::debug!(
					target: LOG_TARGET,
					"Failed to extract data from extrinsic. Transaction: {:?}. Error: {:?}",
					transaction.hash(),
					err,
				),
			};
		}
	}
}
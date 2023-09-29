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

use crate::{Arc, Backend,OffchainDb,warn};
use futures::StreamExt;
use melo_core_primitives::{traits::Extractor, Encode};
use sc_network::NetworkDHTProvider;
use sc_transaction_pool_api::{InPoolTransaction, TransactionPool};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::traits::Block as BlockT;

const LOG_TARGET: &str = "tx_pool_listener";

use crate::{sidecar_kademlia_key, NetworkProvider, Sidecar, SidecarMetadata};

#[derive(Clone)]
pub struct TPListenerParams<Client, Network, TP, BE> {
	pub client: Arc<Client>,
	pub network: Arc<Network>,
	pub transaction_pool: Arc<TP>,
	pub backend: Arc<BE>,
}

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
	tracing::info!(
		target: LOG_TARGET,
		"Starting transaction pool listener.",
	);
	let mut offchain_db = match backend.offchain_storage() {
		Some(offchain_storage) => OffchainDb::new(offchain_storage),
		None => {
			warn!(
				target: LOG_TARGET,
				"Can't spawn a transaction pool listener for a node without offchain storage."
			);
			return
		},
	};
	// Obtain the import notification event stream from the transaction pool
	let mut import_notification_stream = transaction_pool.import_notification_stream();

	// Handle the transaction pool import notification event stream
	while let Some(notification) = import_notification_stream.next().await {
		match transaction_pool.ready_transaction(&notification) {
			Some(transaction) => {
				// TODO: Can we avoid decoding the extrinsic here?
				let encoded = transaction.data().encode();
				let at = client.info().best_hash;
				match client.runtime_api().extract(at, &encoded) {
					Ok(res) => match res {
						Some(data) => {
							data.into_iter().for_each(
								|(data_hash, bytes_len, commitments, proofs)| {
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
										Some(sidecar) => {
											if sidecar.status.is_none() {
												fetch_value_from_network(&sidecar);
											}
										},
										None => {
											let sidecar =
												Sidecar { blobs: None, metadata, status: None };
											sidecar.save_to_local_outside::<B, BE>(&mut offchain_db);
											fetch_value_from_network(&sidecar);
										},
									}
								},
							);
						},
						None => {
							tracing::debug!(
								target: LOG_TARGET,
								"Decoding of extrinsic failed. Transaction: {:?}",
								transaction.hash(),
							);
						},
					},
					Err(err) => {
						tracing::debug!(
							target: LOG_TARGET,
							"Failed to extract data from extrinsic. Transaction: {:?}. Error: {:?}",
							transaction.hash(),
							err,
						);
					},
				};
			},
			None => {},
		}
	}
}

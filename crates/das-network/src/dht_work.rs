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

#![cfg_attr(not(feature = "std"), no_std)]
use crate::{warn, Arc, Backend, Block, DhtEvent, KademliaKey, OffchainDb};
use futures::{channel::mpsc, stream::Fuse, FutureExt, Stream, StreamExt};
use melo_das_primitives::blob::Blob;
use melo_das_primitives::config::FIELD_ELEMENTS_PER_BLOB;
use melo_das_primitives::crypto::KZG;
use melo_erasure_coding::bytes_vec_to_blobs;

/// Logging target for the mmr gadget.
pub const LOG_TARGET: &str = "das-network::dht_work";

use crate::{NetworkProvider, ServicetoWorkerMsg, Sidecar, SidecarStatus};
pub struct Worker<B: Block, Client, Network, DhtEventStream, BE: Backend<B>> {
	#[allow(dead_code)]
	client: Arc<Client>,

	/// Channel receiver for messages send by a [`crate::Service`].
	from_service: Fuse<mpsc::Receiver<ServicetoWorkerMsg>>,

	/// DHT network
	network: Arc<Network>,

	/// Channel we receive Dht events on.
	dht_event_rx: DhtEventStream,

	///
	pub backend: Arc<BE>,

	pub offchain_db: OffchainDb<BE::OffchainStorage>,
}

impl<B, Client, Network, DhtEventStream, BE> Worker<B, Client, Network, DhtEventStream, BE>
where
	B: Block,
	Network: NetworkProvider,
	DhtEventStream: Stream<Item = DhtEvent> + Unpin,
	BE: Backend<B>,
{
	pub(crate) fn try_build(
		from_service: mpsc::Receiver<ServicetoWorkerMsg>,
		client: Arc<Client>,
		backend: Arc<BE>,
		network: Arc<Network>,
		dht_event_rx: DhtEventStream,
	) -> Option<Self> {
		match backend.offchain_storage() {
			Some(offchain_storage) => Some(Worker {
				from_service: from_service.fuse(),
				backend,
				offchain_db: OffchainDb::new(offchain_storage),
				client,
				network,
				dht_event_rx,
			}),
			None => {
				warn!(
					target: LOG_TARGET,
					// TODO
					"Can't spawn a  for a node without offchain storage."
				);
				None
			},
		}
	}

	pub async fn run<FStart>(mut self, start: FStart)
	where
		FStart: Fn(),
	{
		loop {
			start();
			futures::select! {
				event = self.dht_event_rx.next().fuse() => {
					if let Some(event) = event {
						self.handle_dht_event(event).await;
					}
				},
				msg = self.from_service.select_next_some() => {
					self.process_message_from_service(msg);
				},
			}
		}
	}

	async fn handle_dht_event(&mut self, event: DhtEvent) {
		match event {
			DhtEvent::ValueFound(v) => {
				self.handle_dht_value_found_event(v);
			},
			DhtEvent::ValueNotFound(key) => self.handle_dht_value_not_found_event(key),
			// TODO handle other events
			_ => {},
		}
	}

	fn handle_dht_value_found_event(&mut self, values: Vec<(KademliaKey, Vec<u8>)>) {
		for (key, value) in values {
			let maybe_sidecar =
				Sidecar::from_local_outside::<B, BE>(key.as_ref(), &mut self.offchain_db);
			match maybe_sidecar {
				Some(sidecar) => {
					if sidecar.status.is_none() {
						let data_hash = Sidecar::calculate_id(&value);
						let mut new_sidecar = sidecar.clone();
						if data_hash != sidecar.metadata.blobs_hash.as_bytes() {
							new_sidecar.status = Some(SidecarStatus::ProofError);
						} else {
							let kzg = KZG::default_embedded();
							// TODO bytes to blobs
							let blobs = bytes_vec_to_blobs(&[value.clone()], 1).unwrap();
							let encoding_valid = Blob::verify_batch(
								&blobs,
								&sidecar.metadata.commitments,
								&sidecar.metadata.proofs,
								&kzg,
								FIELD_ELEMENTS_PER_BLOB,
							)
							.unwrap();
							if encoding_valid {
								new_sidecar.blobs = Some(value.clone());
								new_sidecar.status = Some(SidecarStatus::Success);
							} else {
								new_sidecar.status = Some(SidecarStatus::ProofError);
							}
						}
						new_sidecar.save_to_local_outside::<B, BE>(&mut self.offchain_db)
					}
				},
				None => {},
			}
		}
	}

	fn handle_dht_value_not_found_event(&mut self, key: KademliaKey) {
		let maybe_sidecar =
			Sidecar::from_local_outside::<B, BE>(key.as_ref(), &mut self.offchain_db);
		match maybe_sidecar {
			Some(sidecar) => {
				if sidecar.status.is_none() {
					let mut new_sidecar = sidecar.clone();
					new_sidecar.status = Some(SidecarStatus::NotFound);
					new_sidecar.save_to_local_outside::<B, BE>(&mut self.offchain_db)
				}
			},
			None => {},
		}
	}

	fn process_message_from_service(&self, msg: ServicetoWorkerMsg) {
		match msg {
			ServicetoWorkerMsg::PutValueToDht(key, value, sender) => {
				let _ = sender.send(Some(self.network.put_value(key, value)));
			},
		}
	}
}

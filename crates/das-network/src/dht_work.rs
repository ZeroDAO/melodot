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

use futures::{channel::mpsc, stream::Fuse, FutureExt, Stream, StreamExt};
use melo_das_primitives::blob::Blob;
use melo_das_primitives::config::FIELD_ELEMENTS_PER_BLOB;
use melo_das_primitives::crypto::KZG;
use melo_erasure_coding::bytes_vec_to_blobs;
use crate::{DhtEvent, KademliaKey};
use std::sync::Arc;

use crate::{NetworkProvider, ServicetoWorkerMsg, Sidercar, SidercarStatus};

pub struct Worker<Client, Network, DhtEventStream> {
	#[allow(dead_code)]
	client: Arc<Client>,

	/// Channel receiver for messages send by a [`crate::Service`].
	from_service: Fuse<mpsc::Receiver<ServicetoWorkerMsg>>,

	/// DHT network
	network: Arc<Network>,

	/// Channel we receive Dht events on.
	dht_event_rx: DhtEventStream,
}

impl<Client, Network, DhtEventStream> Worker<Client, Network, DhtEventStream>
where
	Network: NetworkProvider,
	DhtEventStream: Stream<Item = DhtEvent> + Unpin,
{
	pub(crate) fn new(
		from_service: mpsc::Receiver<ServicetoWorkerMsg>,
		client: Arc<Client>,
		network: Arc<Network>,
		dht_event_rx: DhtEventStream,
	) -> Self {
		Worker { from_service: from_service.fuse(), client, network, dht_event_rx }
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
						Self::handle_dht_event(event).await;
					}
				},
				msg = self.from_service.select_next_some() => {
					self.process_message_from_service(msg);
				},
			}
		}
	}

	async fn handle_dht_event(event: DhtEvent) {
		match event {
			DhtEvent::ValueFound(v) => {
				Self::handle_dht_value_found_event(v);
			},
			DhtEvent::ValueNotFound(key) => Self::handle_dht_value_not_found_event(key),
			// TODO handle other events
			_ => {},
		}
	}

	fn handle_dht_value_found_event(values: Vec<(KademliaKey, Vec<u8>)>) {
		for (key, value) in values {
			let maybe_sidercar = Sidercar::from_local(key.as_ref());
			match maybe_sidercar {
				Some(sidercar) => {
					if sidercar.status.is_none() {
						let data_hash = Sidercar::calculate_id(&value);
						let mut new_sidercar = sidercar.clone();
						if data_hash != sidercar.metadata.blobs_hash.as_bytes() {
							new_sidercar.status = Some(SidercarStatus::ProofError);
						} else {
							let kzg = KZG::default_embedded();
							// TODO bytes to blobs
							let blobs = bytes_vec_to_blobs(&[value.clone()], 1).unwrap();
							let encoding_valid = Blob::verify_batch(
								&blobs,
								&sidercar.metadata.commitments,
								&sidercar.metadata.proofs,
								&kzg,
								FIELD_ELEMENTS_PER_BLOB,
							)
							.unwrap();
							if encoding_valid {
								new_sidercar.blobs = Some(value.clone());
								new_sidercar.status = Some(SidercarStatus::Success);
							} else {
								new_sidercar.status = Some(SidercarStatus::ProofError);
							}
						}
						new_sidercar.save_to_local();
					}
				},
				None => {},
			}
		}
	}

	fn handle_dht_value_not_found_event(key: KademliaKey) {
		let maybe_sidercar = Sidercar::from_local(key.as_ref());
		match maybe_sidercar {
			Some(sidercar) => {
				if sidercar.status.is_none() {
					let mut new_sidercar = sidercar.clone();
					new_sidercar.status = Some(SidercarStatus::NotFound);
					new_sidercar.save_to_local();
				}
			},
			None => {},
		}
	}

	fn process_message_from_service(&self, msg: ServicetoWorkerMsg) {
		match msg {
			ServicetoWorkerMsg::PutValueToDht(key, value) => {
				self.network.put_value(key, value);
			},
		}
	}
}

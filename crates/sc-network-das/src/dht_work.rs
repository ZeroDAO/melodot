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

use futures::Future;
use futures::{FutureExt, Stream, StreamExt};
use melo_core_primitives::blob::Blob;
use melo_core_primitives::config::FIELD_ELEMENTS_PER_BLOB;
use melo_core_primitives::kzg::KZG;
use melo_erasure_coding::bytes_vec_to_blobs;
use sc_network::{DhtEvent, KademliaKey};
use sp_core::blake2_256;
use std::sync::Arc;

use crate::{
	get_sidercar_from_localstorage, save_sidercar_to_localstorage, NetworkProvider, SidercarStatus,
};

pub struct Worker<Client, Network, DhtEventStream> {
	#[warn(dead_code)]
	client: Arc<Client>,

	#[warn(dead_code)]
	network: Arc<Network>,

	/// Channel we receive Dht events on.
	dht_event_rx: DhtEventStream,
	// from_service: Fuse<mpsc::Receiver<ServicetoWorkerMsg<'a>>>,
}

impl<Client, Network, DhtEventStream> Worker<Client, Network, DhtEventStream>
where
	Network: NetworkProvider,
	DhtEventStream: Stream<Item = DhtEvent> + Unpin,
{
	pub fn new(client: Arc<Client>, network: Arc<Network>, dht_event_rx: DhtEventStream) -> Self {
		Worker { client, network, dht_event_rx }
	}

	pub async fn run<Fut, FStart, FHandler>(mut self, start: FStart, handler: FHandler)
	where
		FStart: Fn() -> Fut + Send + Sync + 'static,
		Fut: Future + Send + 'static,
		FHandler: Fn(DhtEvent) -> Fut + Send + Sync + 'static,
	{
		loop {
			start();
			futures::select! {
				event = self.dht_event_rx.next().fuse() => {
					if let Some(event) = event {
						handler(event).await;
					}
				},
			}
		}
	}
}

pub fn handle_dht_event<B, C>(event: DhtEvent) {
	match event {
		DhtEvent::ValueFound(v) => {
			handle_dht_value_found_event(v);
		},
		DhtEvent::ValueNotFound(key) => handle_dht_value_not_found_event(key),
		_ => {},
	}
}

pub fn handle_dht_value_found_event(values: Vec<(KademliaKey, Vec<u8>)>) {
	for (key, value) in values {
		let maybe_sidercar = get_sidercar_from_localstorage(key.as_ref());
		match maybe_sidercar {
			Some(sidercar) => {
				if sidercar.status.is_none() {
					let data_hash = blake2_256(&value);
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
					save_sidercar_to_localstorage(new_sidercar);
				}
			},
			None => {},
		}
	}
}

fn handle_dht_value_not_found_event(key: KademliaKey) {
	let maybe_sidercar = get_sidercar_from_localstorage(key.as_ref());
	match maybe_sidercar {
		Some(sidercar) => {
			if sidercar.status.is_none() {
				let mut new_sidercar = sidercar.clone();
				new_sidercar.status = Some(SidercarStatus::NotFound);
				save_sidercar_to_localstorage(new_sidercar);
			}
		},
		None => {},
	}
}

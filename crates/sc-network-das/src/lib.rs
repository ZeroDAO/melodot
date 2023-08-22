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

use codec::{Decode, Encode};
use frame_system::Config;
// use futures::channel::mpsc;
use futures::{Future, FutureExt, Stream, StreamExt};
use melo_core_primitives::blob::Blob;
use melo_core_primitives::config::FIELD_ELEMENTS_PER_BLOB;
use melo_core_primitives::kzg::{KZGCommitment, KZGProof, KZG};
use melo_erasure_coding::bytes_vec_to_blobs;
use node_primitives::AccountId;
use sc_network::{DhtEvent, KademliaKey, NetworkDHTProvider, NetworkSigner, NetworkStateInfo};
use sc_transaction_pool_api::{InPoolTransaction, TransactionPool};
use serde::{Deserialize, Serialize};
use sp_core::blake2_256;
use sp_core::{offchain::StorageKind, H256};
use sp_runtime::traits::{Block as BlockT, Extrinsic};
use std::sync::Arc;

/// Extracts the `data` field from some types of extrinsics.
pub trait Extractor<T: Extrinsic, AccountId> {
	fn extract(
		app_ext: T,
	) -> Option<(H256, u32, u32, Vec<KZGCommitment>, Vec<KZGProof>, AccountId)>;
}

pub struct Worker<Client, Network, DhtEventStream> {
	client: Arc<Client>,

	network: Arc<Network>,

	/// Channel we receive Dht events on.
	dht_event_rx: DhtEventStream,
}

// Status of the sidercar, including failure to retrieve data and attestation errors
#[derive(Encode, Decode, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SidercarStatus {
    // Failed to retrieve data
    NotFound,
    // Proof error
    ProofError,
    // Successfully retrieved
    Success,
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SidercarMetadata {
    // Data length
    data_len: u32,
    // Hash of the data
    blobs_hash: H256,
    // Commitments
    commitments: Vec<KZGCommitment>,
    // Proofs
    proofs: Vec<KZGProof>,
}

impl SidercarMetadata {
	fn id(&self) -> [u8; 32] {
		Encode::using_encoded(&self.encode(), blake2_256)
	}
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sidercar {
    // User who initiated
    from: AccountId,
    // Metadata
    metadata: SidercarMetadata,
    // Data
    blobs: Option<Vec<u8>>,
    // Status; None means an unhandled edge case and data errors should not be reported at this time
    pub status: Option<SidercarStatus>,
}

impl Sidercar {
	fn id(&self) -> [u8; 32] {
		// Returns hash of sidercar metadata converted to bytes
		self.metadata.id()
	}

	fn kademlia_key(&self) -> KademliaKey {
		KademliaKey::from(Vec::from(self.id()))
	}

	pub fn is_unavailability(&self) -> bool {
		self.status != Some(SidercarStatus::Success) && self.status.is_some()
	}

	// fn hash_blobs(&self) -> Result<[u8; 32], String> {
	// 	if let Some(blobs) = self.blobs {
	// 		let blobs = self.blobs.as_ref().ok_or("blobs is None")?;
	// 		Ok(Encode::using_encoded(&blobs, blake2_256).into())
	// 	} else {
	// 		Err("blobs is None".to_string())
	// 	}
	// }
}

pub trait NetworkProvider: NetworkDHTProvider + NetworkStateInfo + NetworkSigner {}

impl<T> NetworkProvider for T where T: NetworkDHTProvider + NetworkStateInfo + NetworkSigner {}

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

pub async fn run_data_hub<Network, TP, B, C>(transaction_pool: TP, network: Arc<Network>)
where
	B: BlockT + 'static,
	Network: NetworkProvider,
	TP: TransactionPool<Block = B>,
	C: Config + Send + Sync + Extractor<B::Extrinsic, AccountId>,
{
	// Obtain the import notification event stream from the transaction pool
	let mut import_notification_stream = transaction_pool.import_notification_stream();

	// Handle the transaction pool import notification event stream
	while let Some(notification) = import_notification_stream.next().await {
		match transaction_pool.ready_transaction(&notification) {
			Some(transaction) => {
				let extrinsic = transaction.data();
				if let Some((data_hash, bytes_len, _, commitments, proofs, from)) =
					C::extract(extrinsic.clone())
				{
					let metadata = SidercarMetadata {
						data_len: bytes_len,
						blobs_hash: data_hash,
						commitments,
						proofs,
					};

					match get_sidercar_from_localstorage(&metadata.id()) {
						Some(sidercar) => {
							if sidercar.status.is_none() {
								network.get_value(&sidercar.kademlia_key());
							}
						},
						None => {
							let sidercar = Sidercar { from, blobs: None, metadata, status: None };
							save_sidercar_to_localstorage(sidercar.clone());
							network.get_value(&sidercar.kademlia_key());
						},
					}
				}
			},
			None => {},
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

pub fn get_sidercar_from_localstorage(key: &[u8]) -> Option<Sidercar> {
	let maybe_sidercar = sp_io::offchain::local_storage_get(StorageKind::PERSISTENT, key);
	match maybe_sidercar {
		Some(data) => {
			// TODO 错误处理
			let sidercar = Sidercar::decode(&mut &data[..]).unwrap();
			Some(sidercar)
		},
		None => None,
	}
}

fn save_sidercar_to_localstorage(sidercar: Sidercar) {
	sp_io::offchain::local_storage_set(StorageKind::PERSISTENT, &sidercar.id(), &sidercar.encode());
}

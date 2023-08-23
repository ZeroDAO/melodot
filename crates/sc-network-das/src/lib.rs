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

use codec::{Decode, Encode};
use melo_core_primitives::kzg::{KZGCommitment, KZGProof};
use node_primitives::AccountId;
use sc_network::{KademliaKey, NetworkDHTProvider, NetworkSigner, NetworkStateInfo};
use serde::{Deserialize, Serialize};
use sp_core::blake2_256;
use sp_core::{offchain::StorageKind, H256};
use sp_runtime::traits::Extrinsic;

pub mod dht_work;
pub mod tx_pool_listener;

/// Extracts the `data` field from some types of extrinsics.
pub trait Extractor<T: Extrinsic, AccountId> {
	fn extract(
		app_ext: T,
	) -> Option<(H256, u32, u32, Vec<KZGCommitment>, Vec<KZGProof>, AccountId)>;
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

pub fn save_sidercar_to_localstorage(sidercar: Sidercar) {
	sp_io::offchain::local_storage_set(StorageKind::PERSISTENT, &sidercar.id(), &sidercar.encode());
}

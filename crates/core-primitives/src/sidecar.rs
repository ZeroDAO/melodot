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

use crate::localstorage::{get_from_localstorage_with_prefix, save_to_localstorage_with_prefix};
#[cfg(feature = "outside")]
use crate::localstorage::{
	get_from_localstorage_with_prefix_outside, save_to_localstorage_with_prefix_outside,
};
use crate::{String, Vec};
use alloc::format;
use codec::{Decode, Encode};
use melo_das_primitives::{Blob, KZGCommitment, KZGProof, KZG};
use melo_erasure_coding::bytes_to_blobs;
#[cfg(feature = "outside")]
use sc_client_api::Backend;
#[cfg(feature = "outside")]
use sc_offchain::OffchainDb;
#[cfg(feature = "outside")]
use sp_runtime::traits::Block;

use core::result::Result;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_io::hashing;

use melo_das_primitives::config::FIELD_ELEMENTS_PER_BLOB;

const SIDERCAR_PREFIX: &[u8] = b"sidecar";

// Status of the sidecar, including failure to retrieve data and attestation errors
#[derive(Encode, Decode, Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum SidecarStatus {
	// Failed to retrieve data
	NotFound,
	// Proof error
	ProofError,
	// Successfully retrieved
	Success,
}

#[derive(Encode, Debug, Decode, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct SidecarMetadata {
	// Data length
	pub data_len: u32,
	// Hash of the data
	pub blobs_hash: sp_core::H256,
	// Commitments
	pub commitments: Vec<KZGCommitment>,
	// Proofs
	pub proofs: Vec<KZGProof>,
}

impl SidecarMetadata {
	pub fn id(&self) -> [u8; 32] {
		hashing::blake2_256(&self.encode())
	}

	pub fn verify_bytes(&self, bytes: &[u8]) -> Result<bool, String> {
		let kzg = KZG::default_embedded();
		bytes_to_blobs(bytes, FIELD_ELEMENTS_PER_BLOB).and_then(|blobs| {
			Blob::verify_batch(
				&blobs,
				&self.commitments,
				&self.proofs,
				&kzg,
				FIELD_ELEMENTS_PER_BLOB,
			)
		})
	}

	pub fn try_from_app_data(bytes: &[u8]) -> Result<Self, String> {
		let kzg = KZG::default_embedded();

		let data_len = bytes.len() as u32;
		let blobs_hash = Sidecar::calculate_id(bytes);

		let blobs = bytes_to_blobs(bytes, FIELD_ELEMENTS_PER_BLOB)?;

		#[cfg(feature = "std")]
		{
			use rayon::prelude::*;
			let results: Result<Vec<(KZGCommitment, KZGProof)>, String> = blobs
				.par_iter()
				.map(|blob| blob.commit_and_proof(&kzg, FIELD_ELEMENTS_PER_BLOB))
				.collect();

			let (commitments, proofs): (Vec<_>, Vec<_>) = results
				.map_err(|e| format!("Failed to commit and proof: {}", e))?
				.into_iter()
				.unzip();

			Ok(Self { data_len, blobs_hash: blobs_hash.into(), commitments, proofs })
		}

		#[cfg(not(feature = "std"))]
		{
			let mut commitments = Vec::new();
			let mut proofs = Vec::new();

			for blob in blobs.iter() {
				match blob.commit_and_proof(&kzg, FIELD_ELEMENTS_PER_BLOB) {
					Ok((commitment, proof)) => {
						commitments.push(commitment);
						proofs.push(proof);
					},
					Err(e) => return Err(format!("Failed to commit and proof: {}", e)),
				}
			}

			Ok(Self { data_len, blobs_hash: blobs_hash.into(), commitments, proofs })
		}
	}
}

#[derive(Encode, Decode, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct Sidecar {
	// Metadata
	pub metadata: SidecarMetadata,
	// Data
	pub blobs: Option<Vec<u8>>,
	// Status; None means an unhandled edge case and data errors should not be reported at this time
	pub status: Option<SidecarStatus>,
}

impl Sidecar {
	pub fn new(metadata: SidecarMetadata, blobs: Option<Vec<u8>>) -> Self {
		Self { metadata, blobs, status: None }
	}

	pub fn id(&self) -> [u8; 32] {
		// Returns hash of sidecar metadata converted to bytes
		self.metadata.id()
	}

	pub fn calculate_id(blob: &[u8]) -> [u8; 32] {
		hashing::blake2_256(blob)
	}

	pub fn check_hash(&self) -> bool {
		match self.blobs {
			Some(ref blobs) => self.metadata.blobs_hash[..] == Self::calculate_id(blobs),
			None => false,
		}
	}

	pub fn is_unavailability(&self) -> bool {
		self.status != Some(SidecarStatus::Success) && self.status.is_some()
	}

	pub fn set_not_found(&mut self) {
		self.status = Some(SidecarStatus::NotFound);
	}

	pub fn from_local(key: &[u8]) -> Option<Self> {
		let maybe_sidecar = get_from_localstorage_with_prefix(key, SIDERCAR_PREFIX);
		match maybe_sidecar {
			Some(data) => Sidecar::decode(&mut &data[..]).ok(),
			None => None,
		}
	}

	pub fn save_to_local(&self) {
		save_to_localstorage_with_prefix(&self.id(), &self.encode(), SIDERCAR_PREFIX);
	}

	#[cfg(feature = "outside")]
	pub fn from_local_outside<B: Block, BE: Backend<B>>(
		key: &[u8],
		db: &mut OffchainDb<BE::OffchainStorage>,
	) -> Option<Sidecar> {
		let maybe_sidecar =
			get_from_localstorage_with_prefix_outside::<B, BE>(db, key, SIDERCAR_PREFIX);
		match maybe_sidecar {
			Some(data) => Sidecar::decode(&mut &data[..]).ok(),
			None => None,
		}
	}

	#[cfg(feature = "outside")]
	pub fn save_to_local_outside<B: Block, BE: Backend<B>>(
		&self,
		db: &mut OffchainDb<BE::OffchainStorage>,
	) {
		save_to_localstorage_with_prefix_outside::<B, BE>(
			db,
			&self.id(),
			&self.encode(),
			SIDERCAR_PREFIX,
		);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use codec::Encode;
	use sp_core::H256;

	// Mock your `KZGCommitment` and `KZGProof` here if needed

	#[test]
	fn test_sidecar_metadata_id() {
		let metadata = SidecarMetadata {
			data_len: 42,
			blobs_hash: H256::from([1u8; 32]),
			commitments: vec![], // Populate this with real or mocked data
			proofs: vec![],      // Populate this with real or mocked data
		};

		let id = metadata.id();
		assert_eq!(id, hashing::blake2_256(&metadata.encode()));
	}

	#[test]
	fn test_sidecar_new() {
		let metadata = SidecarMetadata {
			data_len: 42,
			blobs_hash: H256::from([1u8; 32]),
			commitments: vec![], // Populate this with real or mocked data
			proofs: vec![],      // Populate this with real or mocked data
		};

		let blobs = Some(vec![1, 2, 3]);
		let sidecar = Sidecar::new(metadata.clone(), blobs.clone());

		assert_eq!(sidecar.metadata, metadata);
		assert_eq!(sidecar.blobs, blobs);
		assert_eq!(sidecar.status, None);
	}

	#[test]
	fn test_sidecar_id() {
		let metadata = SidecarMetadata {
			data_len: 42,
			blobs_hash: H256::from([1u8; 32]),
			commitments: vec![], // Populate this with real or mocked data
			proofs: vec![],      // Populate this with real or mocked data
		};

		let sidecar = Sidecar::new(metadata.clone(), None);
		assert_eq!(sidecar.id(), metadata.id());
	}

	#[test]
	fn test_sidecar_check_hash() {
		let metadata = SidecarMetadata {
			data_len: 3,
			blobs_hash: H256::from(hashing::blake2_256(&[1, 2, 3])),
			commitments: vec![], // Populate this with real or mocked data
			proofs: vec![],      // Populate this with real or mocked data
		};

		let sidecar = Sidecar::new(metadata.clone(), Some(vec![1, 2, 3]));
		assert!(sidecar.check_hash());
	}

	#[test]
	fn test_sidecar_is_unavailability() {
		let metadata = SidecarMetadata {
			data_len: 3,
			blobs_hash: H256::from([1u8; 32]),
			commitments: vec![],
			proofs: vec![],
		};

		let mut sidecar = Sidecar::new(metadata, None);
		sidecar.status = Some(SidecarStatus::NotFound);

		assert!(sidecar.is_unavailability());
	}
}

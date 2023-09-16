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
use crate::{Vec, String};
use alloc::format;

use codec::{Decode, Encode};
use melo_das_primitives::{Blob, KZGCommitment, KZGProof, KZG};
use melo_erasure_coding::bytes_to_blobs;

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_io::hashing;
use core::result::Result;

const SIDERCAR_PREFIX: &[u8] = b"sidercar";

// Status of the sidercar, including failure to retrieve data and attestation errors
#[derive(Encode, Decode, Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum SidercarStatus {
	// Failed to retrieve data
	NotFound,
	// Proof error
	ProofError,
	// Successfully retrieved
	Success,
}

#[derive(Encode, Debug, Decode, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct SidercarMetadata {
	// Data length
	pub data_len: u32,
	// Hash of the data
	pub blobs_hash: sp_core::H256,
	// Commitments
	pub commitments: Vec<KZGCommitment>,
	// Proofs
	pub proofs: Vec<KZGProof>,
}

impl SidercarMetadata {
	pub fn id(&self) -> [u8; 32] {
		hashing::blake2_256(&self.encode())
	}

	pub fn verify_bytes(&self, bytes: &[u8]) -> Result<bool, String> {
		let kzg = KZG::default_embedded();
		let field_elements_per_blob = 4096;
		bytes_to_blobs(bytes, field_elements_per_blob).and_then(|blobs| {
			Blob::verify_batch(
				&blobs,
				&self.commitments,
				&self.proofs,
				&kzg,
				field_elements_per_blob,
			)
		})
	}

	pub fn try_from_app_data(bytes: &[u8]) -> Result<Self, String> {
		let kzg = KZG::default_embedded();
		let field_elements_per_blob = 4096;

		let data_len = bytes.len() as u32;
		let blobs_hash = Sidercar::calculate_id(bytes);

		let blobs = bytes_to_blobs(bytes, 1)?;

		#[cfg(feature = "std")]
		{
			use rayon::prelude::*;
			let results: Result<Vec<(KZGCommitment, KZGProof)>, String> = blobs
				.par_iter()
				.map(|blob| blob.commit_and_proof(&kzg, field_elements_per_blob))
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
				match blob.commit_and_proof(&kzg, field_elements_per_blob) {
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
pub struct Sidercar {
	// Metadata
	pub metadata: SidercarMetadata,
	// Data
	pub blobs: Option<Vec<u8>>,
	// Status; None means an unhandled edge case and data errors should not be reported at this time
	pub status: Option<SidercarStatus>,
}

impl Sidercar {
	pub fn new(metadata: SidercarMetadata, blobs: Option<Vec<u8>>) -> Self {
		Self { metadata, blobs, status: None }
	}

	pub fn id(&self) -> [u8; 32] {
		// Returns hash of sidercar metadata converted to bytes
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
		self.status != Some(SidercarStatus::Success) && self.status.is_some()
	}

	pub fn from_local(key: &[u8]) -> Option<Self> {
		let maybe_sidercar = get_from_localstorage_with_prefix(key, SIDERCAR_PREFIX);
		match maybe_sidercar {
			Some(data) => Sidercar::decode(&mut &data[..]).ok(),
			None => None,
		}
	}

	pub fn save_to_local(&self) {
		save_to_localstorage_with_prefix(&self.id(), &self.encode(), SIDERCAR_PREFIX);
	}

	pub fn set_not_found(&mut self) {
		self.status = Some(SidercarStatus::NotFound);
	}
}

#[cfg(test)]
mod tests {
    use super::*;
    use sp_core::H256;
    use codec::Encode;
    
    // Mock your `KZGCommitment` and `KZGProof` here if needed
    
    #[test]
    fn test_sidercar_metadata_id() {
        let metadata = SidercarMetadata {
            data_len: 42,
            blobs_hash: H256::from([1u8; 32]),
            commitments: vec![],  // Populate this with real or mocked data
            proofs: vec![],  // Populate this with real or mocked data
        };
        
        let id = metadata.id();
        assert_eq!(id, hashing::blake2_256(&metadata.encode()));
    }
    
    #[test]
    fn test_sidercar_new() {
        let metadata = SidercarMetadata {
            data_len: 42,
            blobs_hash: H256::from([1u8; 32]),
            commitments: vec![],  // Populate this with real or mocked data
            proofs: vec![],  // Populate this with real or mocked data
        };
        
        let blobs = Some(vec![1, 2, 3]);
        let sidercar = Sidercar::new(metadata.clone(), blobs.clone());
        
        assert_eq!(sidercar.metadata, metadata);
        assert_eq!(sidercar.blobs, blobs);
        assert_eq!(sidercar.status, None);
    }
    
    #[test]
    fn test_sidercar_id() {
        let metadata = SidercarMetadata {
            data_len: 42,
            blobs_hash: H256::from([1u8; 32]),
            commitments: vec![],  // Populate this with real or mocked data
            proofs: vec![],  // Populate this with real or mocked data
        };
        
        let sidercar = Sidercar::new(metadata.clone(), None);
        assert_eq!(sidercar.id(), metadata.id());
    }
    
    #[test]
    fn test_sidercar_check_hash() {
        let metadata = SidercarMetadata {
            data_len: 3,
            blobs_hash: H256::from(hashing::blake2_256(&[1, 2, 3])),
            commitments: vec![],  // Populate this with real or mocked data
            proofs: vec![],  // Populate this with real or mocked data
        };
        
        let sidercar = Sidercar::new(metadata.clone(), Some(vec![1, 2, 3]));
        assert!(sidercar.check_hash());
    }
    
	#[test]
    fn test_sidercar_is_unavailability() {
        let metadata = SidercarMetadata {
            data_len: 3,
            blobs_hash: H256::from([1u8; 32]),
            commitments: vec![],
            proofs: vec![],
        };

        let mut sidercar = Sidercar::new(metadata, None);
        sidercar.status = Some(SidercarStatus::NotFound);

        assert!(sidercar.is_unavailability());
    }
}

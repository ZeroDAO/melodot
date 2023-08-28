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
use crate::Vec;

use codec::{Decode, Encode};
use melo_das_primitives::{KZGCommitment, KZGProof};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_io::hashing;

const SIDERCAR_PREFIX: &[u8] = b"sidercar";

// Status of the sidercar, including failure to retrieve data and attestation errors
#[derive(Encode, Decode, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum SidercarStatus {
	// Failed to retrieve data
	NotFound,
	// Proof error
	ProofError,
	// Successfully retrieved
	Success,
}

#[derive(Encode, Decode, Clone, PartialEq, Eq)]
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

impl Sidercar
{
	pub fn id(&self) -> [u8; 32] {
		// Returns hash of sidercar metadata converted to bytes
		self.metadata.id()
	}

	pub fn calculate_id(blob: &[u8]) -> [u8; 32] {
		hashing::blake2_256(blob)
	}

	pub fn is_unavailability(&self) -> bool {
		self.status != Some(SidercarStatus::Success) && self.status.is_some()
	}

	pub fn from_local(key: &[u8]) -> Option<Self>
	{
		let maybe_sidercar = get_from_localstorage_with_prefix(key, SIDERCAR_PREFIX);
		match maybe_sidercar {
			Some(data) => Sidercar::decode(&mut &data[..]).ok(),
			None => None,
		}
	}

	pub fn save_to_local(&self)
	{
		save_to_localstorage_with_prefix(&self.id(), &self.encode(), SIDERCAR_PREFIX);
	}
}

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

use crate::{reliability::ReliabilityId, String, TypeInfo, Vec};
use alloc::format;
use codec::{Decode, Encode};
use melo_das_primitives::{Blob, KZGCommitment, KZGProof, KZG};
use melo_erasure_coding::bytes_to_blobs;
use sp_core::RuntimeDebug;

use core::result::Result;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_io::hashing;

use melo_das_primitives::config::FIELD_ELEMENTS_PER_BLOB;

// const SIDERCAR_PREFIX: &[u8] = b"sidecar";

/// Represents the possible statuses of the sidecar, including failures and success cases.
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

/// Contains essential metadata for the sidecar, such as data length, hash, commitments, and proofs.
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
// #[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct SidecarMetadata {
	pub app_id: u32,
	pub bytes_len: u32,
	pub nonce: u32,
	pub commitments: Vec<KZGCommitment>,
	pub proofs: Vec<KZGProof>,
}

impl SidecarMetadata {
	pub fn new(
		app_id: u32,
		bytes_len: u32,
		nonce: u32,
		commitments: Vec<KZGCommitment>,
		proofs: Vec<KZGProof>,
	) -> Self {
		Self { app_id, bytes_len, nonce, commitments, proofs }
	}

	pub fn check(&self) -> bool {
		self.commitments.len() == self.proofs.len() &&
			!self.commitments.is_empty() &&
			self.bytes_len > 0
	}

	pub fn confidence_id(&self) -> ReliabilityId {
		ReliabilityId::app_confidence(self.app_id, self.nonce)
	}

	/// Calculates and returns the ID (hash) of the metadata.
	pub fn id(&self) -> [u8; 32] {
		hashing::blake2_256(&self.encode())
	}

	/// Verifies the provided bytes against the stored commitments and proofs.
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

	/// Attempts to generate a `SidecarMetadata` instance from given application data bytes.
	pub fn try_from_app_data(bytes: &[u8], app_id: u32, nonce: u32) -> Result<Self, String> {
		let kzg = KZG::default_embedded();

		let data_len = bytes.len() as u32;

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

			Ok(Self { app_id, bytes_len: data_len, nonce, commitments, proofs })
		}

		#[cfg(not(feature = "std"))]
		{
			let blob_count = blobs.len();

			let mut commitments = Vec::with_capacity(blob_count);
			let mut proofs = Vec::with_capacity(blob_count);

			for blob in &blobs {
				match blob.commit_and_proof(&kzg, FIELD_ELEMENTS_PER_BLOB) {
					Ok((commitment, proof)) => {
						commitments.push(commitment);
						proofs.push(proof);
					},
					Err(e) => return Err(format!("Failed to commit and proof: {}", e)),
				}
			}

			Ok(Self { app_id, bytes_len: data_len, nonce, commitments, proofs })
		}
	}
}

/// Represents a sidecar, encapsulating its metadata, potential data, and its current status.
#[derive(Encode, Decode, Clone, PartialEq, Eq)]
// #[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct Sidecar {
	/// Metadata associated with the sidecar.
	pub metadata: SidecarMetadata,
	/// Current status of the sidecar; `None` means an unhandled edge case, so data errors
	/// shouldn't be reported.
	pub status: Option<SidecarStatus>,
}

impl Sidecar {
	/// Constructs a new sidecar instance with the provided metadata and data.
	pub fn new(metadata: SidecarMetadata) -> Self {
		Self { metadata, status: None }
	}

	/// Calculates and returns the ID (hash) of the sidecar based on its metadata.
	pub fn id(&self) -> [u8; 32] {
		// Returns hash of sidecar metadata converted to bytes
		self.metadata.id()
	}

	/// Calculates and returns the ID (hash) based on a given blob.
	pub fn calculate_id(blob: &[u8]) -> [u8; 32] {
		hashing::blake2_256(blob)
	}

	/// Determines if the sidecar status represents an unavailability scenario.
	pub fn is_unavailability(&self) -> bool {
		self.status != Some(SidecarStatus::Success) && self.status.is_some()
	}

	/// Sets the status of the sidecar to 'NotFound'.
	pub fn set_not_found(&mut self) {
		self.status = Some(SidecarStatus::NotFound);
	}
}

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
#![recursion_limit = "256"]

extern crate alloc;                                   
pub use alloc::{
	string::{String, ToString},
	vec,
	vec::Vec,
};

pub use melo_das_primitives::{KZGCommitment, KZGProof, Position};
use sp_core::RuntimeDebug;
use sp_runtime::generic::Digest;

#[doc(hidden)]
pub use codec;
#[doc(hidden)]
pub use scale_info;
// #[cfg(feature = "serde")]
// #[doc(hidden)]
// pub use serde;
#[doc(hidden)]
pub use sp_std;

pub mod header;
pub use header::*;

pub mod block;

pub mod sidecar;
pub use sidecar::*;

pub mod config;
pub mod reliability;
pub mod traits;

#[cfg(feature = "serde")]
#[doc(hidden)]
pub(crate) use sp_runtime::serde as runtime_serde;

#[cfg(feature = "std")]
pub mod testing;

/// The SubmitDataParams struct represents parameters for submitting data.
/// It includes the app id, the length of the data, a nonce, a list of commitments, and a list of proofs.
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct SubmitDataParams {
	/// The id of the app.
	pub app_id: u32,
	/// The length of the data to be submitted.
	pub bytes_len: u32,
	/// A nonce for this submission.
	pub nonce: u32,
	/// A list of commitments for this submission.
	pub commitments: Vec<KZGCommitment>,
	/// A list of proofs for this submission.
	pub proofs: Vec<KZGProof>,
}

impl SubmitDataParams {
	/// Creates a new SubmitDataParams with the given parameters.
	pub fn new(
		app_id: u32,
		bytes_len: u32,
		nonce: u32,
		commitments: Vec<KZGCommitment>,
		proofs: Vec<KZGProof>,
	) -> Self {
		Self { app_id, bytes_len, nonce, commitments, proofs }
	}

	/// Checks the validity of the SubmitDataParams.
    /// Returns true if the number of commitments equals the number of proofs,
    /// the commitments are not empty, and the length of the data is greater than 0.
    /// Otherwise, it returns false.
	pub fn check(&self) -> bool {
		self.commitments.len() == self.proofs.len() &&
			!self.commitments.is_empty() &&
			self.bytes_len > 0
	}
}

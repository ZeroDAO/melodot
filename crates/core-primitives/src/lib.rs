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

extern crate alloc;
pub use alloc::{
	string::{String, ToString},
	vec,
	vec::Vec,
};

pub use melo_das_primitives::{KZGCommitment, KZGProof, Position};
use sp_core::RuntimeDebug;
use sp_runtime::generic::Digest;

pub mod header;
pub use header::*;

pub mod sidecar;
pub use sidecar::*;

pub mod config;
pub mod localstorage;
pub mod reliability;
pub mod traits;

#[cfg(feature = "std")]
pub mod testing;

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct SubmitDataParams {
	pub app_id: u32,
	pub bytes_len: u32,
	pub nonce: u32,
	pub commitments: Vec<KZGCommitment>,
	pub proofs: Vec<KZGProof>,
}

impl SubmitDataParams {
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
}

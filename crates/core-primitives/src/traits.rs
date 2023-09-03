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

use crate::{Digest, HeaderExtension, Vec};
use codec::Encode;
use melo_das_primitives::{KZGCommitment, KZGProof};
use sp_core::H256;

pub trait ExtendedHeader {
	/// Header number.
	type Number;

	/// Header hash type
	type Hash;

	/// Creates new header.
	fn new_ext(
		number: Self::Number,
		extrinsics_root: Self::Hash,
		state_root: Self::Hash,
		parent_hash: Self::Hash,
		digest: Digest,
		extension: HeaderExtension,
	) -> Self;

	/// Returns the header extension.
	fn extension(&self) -> &HeaderExtension;

	/// Set the header extension.
	fn set_extension(&mut self, extension: HeaderExtension);

	/// Set the commitment of root.
	fn set_commitments(&mut self, commitment_set: &[KZGCommitment]);

	/// Returns the commitments.
	fn commitments(&self) -> Option<Vec<KZGCommitment>>;

	/// Returns the commitments set bytes.
	fn commitments_bytes(&self) -> &[u8];

	/// Returns the number of columns.
	fn col_num(&self) -> Option<u32>;
}

pub trait HeaderCommitList {
	/// Returns a list of the latest confirmed commitments available for the Blob Matrix.
	///
	/// Note that they are not related to data availability, but rather to the validator's
	/// initial confirmation of the probability of availability.
	fn last() -> Vec<KZGCommitment>;
}

sp_api::decl_runtime_apis! {
	/// Extracts the `data` field from some types of extrinsics.
	pub trait Extractor {
		fn extract(
			extrinsic: &Vec<u8>,
			// (data_hash, bytes_len, commitments, proofs)
		) -> Option<Vec<(H256, u32, Vec<KZGCommitment>, Vec<KZGProof>)>>;
	}
}

sp_api::decl_runtime_apis! {
	pub trait AppDataApi<Call>
	where Call: Encode + Clone {
		fn is_blob_call(
			function: &Vec<u8>,
		) -> Option<bool>;

		fn get_blob_tx_param(
			function: &Call,
		) -> Option<(H256, u32, Vec<KZGCommitment>, Vec<KZGProof>)>;
	}
}

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

use core::fmt::Display;
use alloc::vec::Vec;
use crate::{AppLookup, Digest, HeaderExtension, KZGCommitment, SidecarMetadata};
use codec::{Decode, Encode};
use melo_das_primitives::Position;
use sp_runtime::traits::{
	Block as BlockT, Hash, Header as HeaderT, MaybeSerialize, MaybeSerializeDeserialize,
};
pub trait ExtendedHeader<Number>: HeaderT<Number = Number> {
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
	fn set_extension(&mut self, extension_data: &(Vec<KZGCommitment>, Vec<AppLookup>));

	/// Returns the commitments.
	fn commitments(&self) -> Option<Vec<KZGCommitment>>;

	/// Returns the commitments set bytes.
	fn commitments_bytes(&self) -> &[u8];

	/// Returns the number of columns.
	fn col_num(&self) -> Option<u32>;
}

pub trait ExtendedBlock<Number>:
	BlockT
	+ ExtendedHeaderProvider<
		Number,
		ExtendedHeaderT = <Self as ExtendedBlock<Number>>::ExtendedHeader,
	>
{
	type ExtendedHeader: ExtendedHeader<Number, Hash = Self::Hash> + MaybeSerializeDeserialize;
}

#[doc(hidden)]
pub trait ExtendedHeaderProvider<Number> {
	/// Header type.
	type ExtendedHeaderT: ExtendedHeader<Number>;
}

pub trait HeaderCommitList {
	/// Returns a list of the latest confirmed commitments available for the Blob Matrix.
	///
	/// Note that they are not related to data availability, but rather to the validator's
	/// initial confirmation of the probability of availability.
	fn last() -> (Vec<KZGCommitment>, Vec<AppLookup>);
}

pub trait HeaderWithCommitment: MaybeSerialize + Encode + Sized {
	/// Header number.
	type Number: PartialOrd + Send + Encode + Decode + Copy + Ord + Display;

	/// Header hash type
	type Hash: Encode;

	/// Hashing algorithm
	type Hashing: Hash<Output = Self::Hash>;

	/// Returns the header extension.
	fn extension(&self) -> &HeaderExtension;

	/// Returns the commitments.
	fn commitments(&self) -> Option<Vec<KZGCommitment>>;

	/// Returns the commitments set bytes.
	fn commitments_bytes(&self) -> &[u8];

	/// Returns the number of columns.
	fn col_num(&self) -> Option<u32>;

	fn number(&self) -> &Self::Number;

	/// Returns the hash of the header.
	fn hash(&self) -> Self::Hash {
		<Self::Hashing as Hash>::hash_of(self)
	}
}

sp_api::decl_runtime_apis! {
	/// Extracts the `data` field from some types of extrinsics.
	#[allow(clippy::ptr_arg, clippy::type_complexity)]
	pub trait Extractor {
		fn extract(
			extrinsic: &Vec<u8>,
			// (data_hash, bytes_len, commitments, proofs)
		) -> Option<Vec<SidecarMetadata>>;
	}
}

sp_api::decl_runtime_apis! {
	pub trait AppDataApi<RuntimeCall>
	where RuntimeCall: Encode {
		fn get_blob_tx_param(
			function: &RuntimeCall,
		) -> Option<SidecarMetadata>;
	}
}

pub trait CommitmentFromPosition {
	type BlockNumber;

	fn commitments(block_number: Self::BlockNumber, postion: &Position) -> Option<KZGCommitment>;
}

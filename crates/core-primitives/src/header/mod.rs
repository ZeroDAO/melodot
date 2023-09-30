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

//! Generic implementation of a block header.
//! This is an extension of the Substrate default header
//! https://github.com/paritytech/substrate/blob/master/primitives/runtime/src/generic/header.rs

use crate::Vec;

pub use codec::{Codec, Decode, Encode};
pub use scale_info::TypeInfo;

use crate::traits::ExtendedHeader;
use crate::Digest;

pub mod extension;
pub use extension::HeaderExtension;

use melo_das_primitives::KZGCommitment;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_core::U256;
use sp_runtime::traits::{
	self, AtLeast32BitUnsigned, Hash as HashT, MaybeDisplay, MaybeSerialize,
	MaybeSerializeDeserialize, Member, SimpleBitOps,
};
use sp_std::fmt::Debug;

/// Abstraction over a block header for a substrate chain.
#[derive(Encode, Decode, PartialEq, Eq, Clone, sp_core::RuntimeDebug, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
#[cfg_attr(feature = "std", serde(deny_unknown_fields))]
pub struct Header<Number: Copy + Into<U256> + TryFrom<U256>, Hash: HashT> {
	/// The parent hash.
	pub parent_hash: Hash::Output,
	/// The block number.
	#[cfg_attr(
		feature = "std",
		serde(serialize_with = "serialize_number", deserialize_with = "deserialize_number")
	)]
	#[codec(compact)]
	pub number: Number,
	/// The state trie merkle root
	pub state_root: Hash::Output,
	/// The merkle root of the extrinsics.
	pub extrinsics_root: Hash::Output,
	/// A chain-specific digest of data useful for light clients or referencing auxiliary data.
	pub digest: Digest,
	/// Extension data.
	pub extension: HeaderExtension,
}

#[cfg(feature = "std")]
pub fn serialize_number<S, T: Copy + Into<U256> + TryFrom<U256>>(
	val: &T,
	s: S,
) -> Result<S::Ok, S::Error>
where
	S: serde::Serializer,
{
	let u256: U256 = (*val).into();
	serde::Serialize::serialize(&u256, s)
}

#[cfg(feature = "std")]
pub fn deserialize_number<'a, D, T: Copy + Into<U256> + TryFrom<U256>>(d: D) -> Result<T, D::Error>
where
	D: serde::Deserializer<'a>,
{
	let u256: U256 = serde::Deserialize::deserialize(d)?;
	TryFrom::try_from(u256).map_err(|_| serde::de::Error::custom("Try from failed"))
}

impl<Number, Hash> traits::Header for Header<Number, Hash>
where
	Number: Member
		+ MaybeSerializeDeserialize
		+ Debug
		+ sp_std::hash::Hash
		+ MaybeDisplay
		+ AtLeast32BitUnsigned
		+ Codec
		+ Copy
		+ Into<U256>
		+ TryFrom<U256>
		+ sp_std::str::FromStr,
	Hash: HashT,
	Hash::Output: Default
		+ sp_std::hash::Hash
		+ Copy
		+ Member
		+ Ord
		+ MaybeSerialize
		+ Debug
		+ MaybeDisplay
		+ SimpleBitOps
		+ Codec,
{
	type Number = Number;
	type Hash = <Hash as HashT>::Output;
	type Hashing = Hash;

	fn number(&self) -> &Self::Number {
		&self.number
	}
	fn set_number(&mut self, num: Self::Number) {
		self.number = num
	}

	fn extrinsics_root(&self) -> &Self::Hash {
		&self.extrinsics_root
	}
	fn set_extrinsics_root(&mut self, root: Self::Hash) {
		self.extrinsics_root = root
	}

	fn state_root(&self) -> &Self::Hash {
		&self.state_root
	}
	fn set_state_root(&mut self, root: Self::Hash) {
		self.state_root = root
	}

	fn parent_hash(&self) -> &Self::Hash {
		&self.parent_hash
	}
	fn set_parent_hash(&mut self, hash: Self::Hash) {
		self.parent_hash = hash
	}

	fn digest(&self) -> &Digest {
		&self.digest
	}

	fn digest_mut(&mut self) -> &mut Digest {
		#[cfg(feature = "std")]
		log::debug!(target: "header", "Retrieving mutable reference to digest");
		&mut self.digest
	}

	fn new(
		number: Self::Number,
		extrinsics_root: Self::Hash,
		state_root: Self::Hash,
		parent_hash: Self::Hash,
		digest: Digest,
	) -> Self {
		Self {
			number,
			extrinsics_root,
			state_root,
			parent_hash,
			digest,
			extension: Default::default(),
		}
	}
}

impl<Number, Hash> Header<Number, Hash>
where
	Number: Member
		+ sp_std::hash::Hash
		+ Copy
		+ MaybeDisplay
		+ AtLeast32BitUnsigned
		+ Codec
		+ Into<U256>
		+ TryFrom<U256>,
	Hash: HashT,
	Hash::Output:
		Default + sp_std::hash::Hash + Copy + Member + MaybeDisplay + SimpleBitOps + Codec,
{
	/// Convenience helper for computing the hash of the header without having
	/// to import the trait.
	pub fn hash(&self) -> Hash::Output {
		Hash::hash_of(self)
	}
}

impl<Number: Copy + Into<U256> + TryFrom<U256>, Hash: HashT> ExtendedHeader
	for Header<Number, Hash>
{
	type Number = Number;
	type Hash = <Hash as HashT>::Output;

	/// Creates new header.
	fn new_ext(
		number: Self::Number,
		extrinsics: Self::Hash,
		state: Self::Hash,
		parent: Self::Hash,
		digest: Digest,
		extension: HeaderExtension,
	) -> Self {
		Self {
			parent_hash: parent,
			number,
			state_root: state,
			extrinsics_root: extrinsics,
			digest,
			extension,
		}
	}

	fn extension(&self) -> &HeaderExtension {
		&self.extension
	}

	fn set_extension(&mut self, extension: HeaderExtension) {
		self.extension = extension;
	}

	fn set_commitments(&mut self, commitment_set: &[KZGCommitment]) {
		self.extension.commitments_bytes =
			commitment_set.iter().flat_map(|c| c.to_bytes()).collect::<Vec<u8>>();
	}

	fn commitments(&self) -> Option<Vec<KZGCommitment>> {
		let result: Result<Vec<KZGCommitment>, _> = self
			.extension
			.commitments_bytes
			.chunks(KZGCommitment::size())
			.map(|c| Decode::decode(&mut &c[..]))
			.collect();

		result.ok()
	}

	fn commitments_bytes(&self) -> &[u8] {
		&self.extension.commitments_bytes
	}

	fn col_num(&self) -> Option<u32> {
		(self.extension.commitments_bytes.len() / KZGCommitment::size()).try_into().ok()
	}
}

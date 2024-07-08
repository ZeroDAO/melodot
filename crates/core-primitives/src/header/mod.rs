// This file is part of Substrate.

// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Generic implementation of a block header.

//! Generic implementation of a block header.
//! This is an extension of the Substrate default header
//! https://github.com/paritytech/substrate/blob/master/primitives/runtime/src/generic/header.rs

// use crate::Vec;

use core::fmt::Display;

pub use codec::{Codec, Decode, Encode, EncodeLike};
pub use scale_info::TypeInfo;

use crate::{
	traits::{ExtendedHeader, HeaderWithCommitment},
	Digest,
};
use crate::Vec;

pub mod extension;
pub use extension::{AppLookup, HeaderExtension};

#[cfg(feature = "serde")]
use crate::runtime_serde;
use melo_das_primitives::KZGCommitment;
use sp_core::U256;
use sp_runtime::traits::{
	self, AtLeast32BitUnsigned, BlockNumber, Hash as HashT, MaybeDisplay, Member, SimpleBitOps,
};

/// Abstraction over a block header for a substrate chain.
#[derive(Encode, Decode, PartialEq, Eq, Clone, sp_core::RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(Hash))]
#[cfg_attr(feature = "serde", derive(runtime_serde::Serialize, runtime_serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct Header<Number: Copy + Into<U256> + TryFrom<U256>, Hash: HashT> {
	/// The parent hash.
	pub parent_hash: Hash::Output,
	/// The block number.
	#[cfg_attr(
		feature = "serde",
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

#[cfg(feature = "serde")]
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

#[cfg(feature = "serde")]
pub fn deserialize_number<'a, D, T: Copy + Into<U256> + TryFrom<U256>>(d: D) -> Result<T, D::Error>
where
	D: serde::Deserializer<'a>,
{
	let u256: U256 = serde::Deserialize::deserialize(d)?;
	TryFrom::try_from(u256).map_err(|_| serde::de::Error::custom("Try from failed"))
}

impl<Number, Hash> traits::Header for Header<Number, Hash>
where
	Number: BlockNumber,
	Hash: HashT,
{
	type Number = Number;
	type Hash = <Hash as HashT>::Output;
	type Hashing = Hash;

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
}

impl<Number, Hash> Header<Number, Hash>
where
	Number: Member
		+ core::hash::Hash
		+ Copy
		+ MaybeDisplay
		+ AtLeast32BitUnsigned
		+ Codec
		+ Into<U256>
		+ TryFrom<U256>,
	Hash: HashT,
{
	/// Convenience helper for computing the hash of the header without having
	/// to import the trait.
	pub fn hash(&self) -> Hash::Output {
		Hash::hash_of(self)
	}
}

impl<Number, Hash> ExtendedHeader<Number> for Header<Number, Hash>
where
	Number: BlockNumber,
	Hash: HashT,
{
	/// Creates new header.
	fn new_ext(
		number: Number,
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

	fn set_extension(&mut self, extension_data: &(Vec<KZGCommitment>, Vec<AppLookup>)) {
		let (commitments, app_lookups) = extension_data;
		self.extension.commitments_bytes =
			commitments.iter().flat_map(|c| c.to_bytes()).collect::<Vec<u8>>();
		self.extension.app_lookup = app_lookups.clone();
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

impl<Number, Hash> HeaderWithCommitment for Header<Number, Hash>
where
	Number: Member
		+ sp_std::hash::Hash
		+ Copy
		+ MaybeDisplay
		+ AtLeast32BitUnsigned
		+ Codec
		+ Into<U256>
		+ TryFrom<U256>
		+ PartialOrd
		+ Display,
	Hash: HashT,
	Hash::Output:
		Default + sp_std::hash::Hash + Copy + Member + MaybeDisplay + SimpleBitOps + Codec,
{
	type Number = Number;
	type Hash = <Hash as HashT>::Output;
	type Hashing = Hash;

	fn extension(&self) -> &HeaderExtension {
		&self.extension
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

	fn number(&self) -> &Self::Number {
		&self.number
	}

	fn hash(&self) -> Hash::Output {
		Hash::hash_of(self)
	}
}

// #[cfg(all(test, feature = "std"))]
// mod tests {
// 	use super::*;
// 	use crate::traits::BlakeTwo256;

// 	#[test]
// 	fn should_serialize_numbers() {
// 		fn serialize(num: u128) -> String {
// 			let mut v = vec![];
// 			{
// 				let mut ser = serde_json::Serializer::new(std::io::Cursor::new(&mut v));
// 				serialize_number(&num, &mut ser).unwrap();
// 			}
// 			String::from_utf8(v).unwrap()
// 		}

// 		assert_eq!(serialize(0), "\"0x0\"".to_owned());
// 		assert_eq!(serialize(1), "\"0x1\"".to_owned());
// 		assert_eq!(serialize(u64::MAX as u128), "\"0xffffffffffffffff\"".to_owned());
// 		assert_eq!(serialize(u64::MAX as u128 + 1), "\"0x10000000000000000\"".to_owned());
// 	}

// 	#[test]
// 	fn should_deserialize_number() {
// 		fn deserialize(num: &str) -> u128 {
// 			let mut der = serde_json::Deserializer::new(serde_json::de::StrRead::new(num));
// 			deserialize_number(&mut der).unwrap()
// 		}

// 		assert_eq!(deserialize("\"0x0\""), 0);
// 		assert_eq!(deserialize("\"0x1\""), 1);
// 		assert_eq!(deserialize("\"0xffffffffffffffff\""), u64::MAX as u128);
// 		assert_eq!(deserialize("\"0x10000000000000000\""), u64::MAX as u128 + 1);
// 	}

// 	#[test]
// 	fn ensure_format_is_unchanged() {
// 		let header = Header::<u32, BlakeTwo256> {
// 			parent_hash: BlakeTwo256::hash(b"1"),
// 			number: 2,
// 			state_root: BlakeTwo256::hash(b"3"),
// 			extrinsics_root: BlakeTwo256::hash(b"4"),
// 			digest: crate::generic::Digest {
// 				logs: vec![crate::generic::DigestItem::Other(b"6".to_vec())],
// 			},
// 			extension: Default::default(),
// 		};

// 		let header_encoded = header.encode();
// 		assert_eq!(
// 			header_encoded,
// 			vec![
// 				146, 205, 245, 120, 196, 112, 133, 165, 153, 34, 86, 240, 220, 249, 125, 11, 25,
// 				241, 241, 201, 222, 77, 95, 227, 12, 58, 206, 97, 145, 182, 229, 219, 8, 88, 19,
// 				72, 51, 123, 15, 62, 20, 134, 32, 23, 61, 170, 165, 249, 77, 0, 216, 129, 112, 93,
// 				203, 240, 170, 131, 239, 218, 186, 97, 210, 237, 225, 235, 134, 73, 33, 73, 151,
// 				87, 78, 32, 196, 100, 56, 138, 23, 36, 32, 210, 84, 3, 104, 43, 187, 184, 12, 73,
// 				104, 49, 200, 204, 31, 143, 13, 4, 0, 4, 54
// 			],
// 		);
// 		assert_eq!(header, Header::<u32, BlakeTwo256>::decode(&mut &header_encoded[..]).unwrap());

// 		let header = Header::<u32, BlakeTwo256> {
// 			parent_hash: BlakeTwo256::hash(b"1000"),
// 			number: 2000,
// 			state_root: BlakeTwo256::hash(b"3000"),
// 			extrinsics_root: BlakeTwo256::hash(b"4000"),
// 			digest: crate::generic::Digest {
// 				logs: vec![crate::generic::DigestItem::Other(b"5000".to_vec())],
// 			},
// 			extension: Default::default(),
// 		};

// 		let header_encoded = header.encode();
// 		assert_eq!(
// 			header_encoded,
// 			vec![
// 				197, 243, 254, 225, 31, 117, 21, 218, 179, 213, 92, 6, 247, 164, 230, 25, 47, 166,
// 				140, 117, 142, 159, 195, 202, 67, 196, 238, 26, 44, 18, 33, 92, 65, 31, 219, 225,
// 				47, 12, 107, 88, 153, 146, 55, 21, 226, 186, 110, 48, 167, 187, 67, 183, 228, 232,
// 				118, 136, 30, 254, 11, 87, 48, 112, 7, 97, 31, 82, 146, 110, 96, 87, 152, 68, 98,
// 				162, 227, 222, 78, 14, 244, 194, 120, 154, 112, 97, 222, 144, 174, 101, 220, 44,
// 				111, 126, 54, 34, 155, 220, 253, 124, 4, 0, 16, 53, 48, 48, 48
// 			],
// 		);
// 		assert_eq!(header, Header::<u32, BlakeTwo256>::decode(&mut &header_encoded[..]).unwrap());
// 	}
// }

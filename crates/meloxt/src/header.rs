// Copyright 2023 ZeroDAO

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at

//     http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use codec::{Decode, Encode};
pub use primitive_types::{H256, U256};
use serde::{Deserialize, Serialize};
use subxt::config::{substrate::Digest, Hasher, Header as SPHeader};

use melo_core_primitives::HeaderExtension;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
#[serde(rename_all = "camelCase")]
pub struct MelodotHeader<N: Copy + Into<U256> + TryFrom<U256>, H: Hasher> {
	/// The parent hash of this block.
	pub parent_hash: H::Output,
	/// The block number.
	#[serde(serialize_with = "serialize_number", deserialize_with = "deserialize_number")]
	#[codec(compact)]
	pub number: N,
	/// The state trie merkle root of this block.
	pub state_root: H::Output,
	/// The extrinsics trie merkle root of this block.
	pub extrinsics_root: H::Output,
	/// The digest of this block.
	pub digest: Digest,
	/// The commitment list of this block.
	pub extension: HeaderExtension,
}

fn serialize_number<S, T: Copy + Into<U256>>(val: &T, s: S) -> Result<S::Ok, S::Error>
where
	S: serde::Serializer,
{
	let u256: U256 = (*val).into();
	serde::Serialize::serialize(&u256, s)
}

fn deserialize_number<'a, D, T: TryFrom<U256>>(d: D) -> Result<T, D::Error>
where
	D: serde::Deserializer<'a>,
{
	// At the time of writing, Smoldot gives back block numbers in numeric rather
	// than hex format. So let's support deserializing from both here:
	use subxt::rpc::types::NumberOrHex;
	let number_or_hex = NumberOrHex::deserialize(d)?;
	let u256 = number_or_hex.into_u256();
	TryFrom::try_from(u256).map_err(|_| serde::de::Error::custom("Try from failed"))
}

impl<N, H> SPHeader for MelodotHeader<N, H>
where
	N: Copy + Into<u64> + Into<U256> + TryFrom<U256> + Encode,
	H: Hasher + Encode,
	MelodotHeader<N, H>: Encode + Decode,
{
	type Number = N;
	type Hasher = H;

	fn number(&self) -> Self::Number {
		self.number
	}
}

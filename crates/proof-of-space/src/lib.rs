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

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub(crate) use chacha20::{
	cipher::{KeyIvInit, StreamCipher},
	ChaCha8, Nonce,
};
use codec::{Decode, Encode};
use melo_core_primitives::config::EXTENDED_SEGMENTS_PER_BLOB;
use melo_das_db::traits::DasKv;
use melo_das_primitives::Segment;
use scale_info::TypeInfo;
pub use sp_core::H256;

pub(crate) use alloc::vec::Vec;
pub(crate) use sp_runtime::traits::{BlakeTwo256, Hash as HashT};

#[cfg(feature = "std")]
pub mod mock;

pub mod cell;
pub mod piece;
pub mod solution;
pub mod utils;
pub mod y_value_manager;
pub mod z_value_manager;

pub use cell::{Cell, CellMetadata, PreCell};
pub use piece::{Piece, PieceMetadata, PiecePosition};
#[cfg(feature = "std")]
pub use solution::find_solutions;
pub use solution::Solution;
pub use y_value_manager::{XValueManager, YPos};
pub use z_value_manager::ZValueManager;

#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug, TypeInfo)]
pub struct FarmerId(pub H256);

impl From<H256> for FarmerId {
	fn from(h: H256) -> Self {
		Self(h)
	}
}

impl From<FarmerId> for H256 {
	fn from(f: FarmerId) -> Self {
		f.0
	}
}

impl Default for FarmerId {
	fn default() -> Self {
		Self(H256::default())
	}
}

impl AsRef<H256> for FarmerId {
	fn as_ref(&self) -> &H256 {
		&self.0
	}
}

impl AsRef<[u8]> for FarmerId {
	fn as_ref(&self) -> &[u8] {
		self.0.as_ref()
	}
}

impl FarmerId {
	pub fn new<T: Encode>(t: T) -> Self {
		let encoded = t.encode();
		if encoded.iter().all(|&byte| byte == 0) {
			FarmerId(H256::default())
		} else {
			let h256 = BlakeTwo256::hash(&encoded);
			FarmerId(h256)
		}
	}
}

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
use melo_core_primitives::config::FIELD_ELEMENTS_PER_SEGMENT;
use melo_das_db::traits::DasKv;
use melo_das_primitives::{BlsScalar, KZGProof, Position, Segment};
pub use sp_core::H256;

pub(crate) use sp_runtime::traits::{BlakeTwo256, Hash as HashT};

pub mod cell;
pub mod piece;
pub mod solution;
pub mod utils;
pub mod y_value_manager;
pub mod z_value_manager;

pub use cell::{Cell, PreCell, CellMetadata};
pub use piece::{Piece, PieceMetadata, PiecePosition};
pub use solution::{find_solutions, Solution};
pub use y_value_manager::{XValueManager, YPos};
pub use z_value_manager::ZValueManager;

pub type FarmerId = H256;

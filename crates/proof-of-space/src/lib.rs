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

use core::marker::PhantomData;
use alloc::vec::Vec;
pub(crate) use chacha20::{
    cipher::{KeyIvInit, StreamCipher},
    ChaCha8, Key, Nonce,
};
use codec::{Decode, Encode};
use melo_das_db::traits::DasKv;
use melo_das_primitives::{KZGProof, Position, Segment};
pub use sp_core::H256;

pub(crate) use sp_runtime::traits::{BlakeTwo256, Hash as HashT};

pub mod cell_metadata;
pub mod segment_entity;
pub mod solution;
pub mod utils;
pub mod x_value_manager;

use cell_metadata::CellMetadata;
use x_value_manager::XValueManager;

pub use segment_entity::SegmentEntity;
pub use solution::{find_solutions, Cell, Solution};

pub type FarmerId = H256;

/// `Record` is a struct that represents a record in the system.
/// It contains a `Segment` and a `block_num`.
///
/// Type parameters:
/// * `BlockNumber`: The type representing the block number.
#[derive(Debug, Default, Clone, PartialEq, Eq, Encode, Decode)]
pub struct Record<BlockNumber> {
    pub seg: Segment,
    pub block_num: BlockNumber,
}

/// `Table` is a generic struct representing a table in the database.
/// It stores a record identifier, a final X value, and uses phantom data for generic types.
///
/// Type parameters:
/// * `H`: A trait bound for a hash type (implementing `HashT`).
/// * `BlockNumber`: The type for block numbers, which must be cloneable and hashable.
pub struct Table<H: HashT, BlockNumber>
where
    BlockNumber: Clone + sp_std::hash::Hash,
{
    pub record_id: H256,
    pub final_x: u32,
    _block_num: PhantomData<BlockNumber>,
    _hash: PhantomData<H>,
}

impl<H: HashT, BlockNumber> Table<H, BlockNumber>
where
    BlockNumber: Clone + sp_std::hash::Hash + Encode + Decode,
{
    /// Creates a new `Table` instance with the given `record_id`.
    ///
    /// Parameters:
    /// * `record_id`: The H256 identifier of the record.
    ///
    /// Returns:
    /// A new instance of `Table`.
    pub fn new(record_id: H256) -> Self {
        Self { 
            record_id, 
            final_x: 0u32, 
            _block_num: PhantomData, 
            _hash: PhantomData,
        }
    }

    /// Generates a vector of tuples containing `final_x` and `CellMetadata` for each cell.
    ///
    /// Parameters:
    /// * `seg`: A reference to a `Segment`.
    /// * `farmer_id`: A reference to a `FarmerId`.
    /// * `block_num`: The block number.
    ///
    /// Returns:
    /// A `Vec<(u32, CellMetadata)>` where each tuple represents a cell's final X value and its metadata.
    pub fn cells(
        &self,
        seg: &Segment,
        farmer_id: &FarmerId,
        block_num: BlockNumber,
    ) -> Vec<(u32, CellMetadata)>
    where
        BlockNumber: Clone + Encode,
    {
        let mut cell_index = CellMetadata::new(self.record_id);
        seg.content
            .data
            .chunks(32)
            .enumerate()
            .map(|(i, _)| {
                cell_index.index = i as u32;
                let final_x = XValueManager::<H, BlockNumber>::new(
                    farmer_id.clone(),
                    block_num.clone(),
                    seg.position.clone(),
                    i as u32,
                )
                .calculate_final_x(16);
                (final_x, cell_index.clone())
            })
            .collect()
    }
}

/// Computes and returns the record ID based on position and block number.
///
/// Parameters:
/// * `position`: A reference to a `Position`.
/// * `block_num`: A reference to the block number.
///
/// Returns:
/// The computed `H256` record ID.
pub fn get_record_id<BlockNumber>(position: &Position, block_num: &BlockNumber) -> H256
where
    BlockNumber: Clone + Encode,
{
    let mut position = position.encode();
    let mut block_num = block_num.encode();
    position.append(&mut block_num);
    BlakeTwo256::hash_of(&position)
}

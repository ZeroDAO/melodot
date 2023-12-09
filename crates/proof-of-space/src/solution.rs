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

use crate::{
	utils, BlakeTwo256, Cell, DasKv, Decode, Encode, FarmerId, HashT, Piece, PreCell, ZValueManager,
};
use anyhow::{Ok, Result};
use melo_das_primitives::{KZGCommitment, KZG};
use scale_info::TypeInfo;

/// `Solution` represents a potential solution in the system.
#[derive(Debug, Default, Clone, TypeInfo)]
pub struct Solution<Hash, BlockNumber>
where
	BlockNumber: Clone + sp_std::hash::Hash + Encode + Decode,
	Hash: PartialEq + Eq + AsRef<[u8]> + 'static,
{
	// The hash of the block that posted the solution.
	block_hash: Hash,
	// The ID of the farmer.
	farmer_id: FarmerId,
	// The previous cell. This is the cell that in the posted block.
	pre_cell: PreCell,
	// The winning cell.
	win_cell_left: Cell<BlockNumber>,
	// The winning cell.
	win_cell_right: Cell<BlockNumber>,
}

impl<Hash, BlockNumber> Solution<Hash, BlockNumber>
where
	BlockNumber: Clone + sp_std::hash::Hash + Encode + Decode + PartialEq,
	Hash: PartialEq + Eq + AsRef<[u8]> + Clone + 'static,
{
	/// Creates a new solution.
	pub fn new(
		block_hash: &Hash,
		farmer_id: &FarmerId,
		pre_cell: &PreCell,
		win_cell_left: &Cell<BlockNumber>,
		win_cell_right: &Cell<BlockNumber>,
	) -> Self {
		Self {
			block_hash: block_hash.clone(),
			farmer_id: *farmer_id,
			pre_cell: pre_cell.clone(),
			win_cell_left: win_cell_left.clone(),
			win_cell_right: win_cell_right.clone(),
		}
	}

	/// Verifies the correctness of the solution.
	#[allow(clippy::too_many_arguments)]
	pub fn verify(
		&self,
		pre_commit: &KZGCommitment,
		win_left_commit: &KZGCommitment,
		win_right_commit: &KZGCommitment,
		win_left_block_hash: &Hash,
		win_right_block_hash: &Hash,
		pre_cell_leading_zero: u8,
		n: u32,
	) -> bool {
		let kzg = KZG::default_embedded();
		Self::check_pre_cell(&self.pre_cell.data, &self.farmer_id, pre_cell_leading_zero) &&
			Self::is_index_valid(
				&self.farmer_id,
				&self.block_hash,
				self.pre_cell.piece_index() as usize,
				32,
				n,
			) && self.pre_cell.verify_kzg_proof(&kzg, pre_commit) &&
			self.validate_win_cell(
				&kzg,
				win_left_commit,
				win_right_commit,
				win_left_block_hash,
				win_right_block_hash,
				n,
			)
	}

	pub fn check_pre_cell(
		cell_data: &[u8; 31],
		farmer_id: &FarmerId,
		pre_cell_leading_zero: u8,
	) -> bool {
		let pre_cell_hash = BlakeTwo256::hash_of(cell_data);
		let xored_hash = utils::xor_byte_slices(farmer_id.as_ref(), pre_cell_hash.as_ref());

		utils::validate_leading_zeros(&xored_hash, pre_cell_leading_zero as u32)
	}

	pub fn is_index_valid(
		farmer_id: &FarmerId,
		block_hash: &Hash,
		index: usize,
		max_index: usize,
		n: u32,
	) -> bool {
		let xored_hash = utils::xor_byte_slices(farmer_id.as_ref(), block_hash.as_ref());
		utils::is_index_valid(&xored_hash, index, max_index, n as usize)
	}

	/// Selects a set of indices based on a XORed hash.
	///
	/// This function computes indices based on the XORed hash derived from
	/// `farmer_id` and `block_hash`. The selection is influenced by the
	/// 'stretch factor' `n`, with higher values of `n` resulting in a lower
	/// probability of index selection. The `end` parameter determines the
	/// maximum index that can be selected.
	///
	/// # Arguments
	///
	/// * `farmer_id` - A reference to the FarmerId, used as part of the hash input.
	/// * `block_hash` - A reference to the Hash::Output, used as the other part of the hash input.
	/// * `end` - The maximum index that can be considered for selection.
	/// * `n` - The stretch factor. Higher values decrease the probability of each index being
	///   selected.
	///
	/// # Returns
	///
	/// A vector of selected indices.
	pub fn select_indices(
		farmer_id: &FarmerId,
		block_hash: &Hash,
		end: usize,
		n: usize,
	) -> Vec<u32> {
		utils::select_indices(
			&utils::xor_byte_slices(farmer_id.as_ref(), block_hash.as_ref())
				.try_into()
				.expect("Expected a 32-byte array"),
			0,
			end,
			n,
		)
	}

	// Validates the winning cell.
	fn validate_win_cell(
		&self,
		kzg: &KZG,
		win_left_commit: &KZGCommitment,
		win_right_commit: &KZGCommitment,
		win_left_block_hash: &Hash,
		win_right_block_hash: &Hash,
		n: u32,
	) -> bool {
		let z = ZValueManager::<BlockNumber>::get_challenge(self.block_hash.as_ref());
		utils::is_index_valid(
			win_left_block_hash.as_ref(),
			self.win_cell_left.piece_index() as usize,
			32,
			n as usize,
		) && utils::is_index_valid(
			win_right_block_hash.as_ref(),
			self.win_cell_right.piece_index() as usize,
			32,
			n as usize,
		) && self.win_cell_left.verify_kzg_proof(kzg, win_left_commit) &&
			self.win_cell_right.verify_kzg_proof(kzg, win_right_commit) &&
			ZValueManager::<BlockNumber>::verify(
				z,
				&self.farmer_id,
				&self.win_cell_left.data.into(),
				&self.win_cell_right.data.into(),
				&self.win_cell_left.metadata,
				&self.win_cell_right.metadata,
			)
	}
}

/// Finds solutions in the database and returns a tuple containing the winning cell and its nonce.
/// The nonce is used to generate the key for the ChaCha8 stream cipher.
/// The function returns a vector of tuples containing the winning cell and its nonce.
///
/// Parameters:
/// * `db`: A mutable reference to an object that implements the `DasKv` trait.
/// * `farmer_id`: A reference to a `FarmerId`.
/// * `block_num`: The block number of block that posted the solution.
/// * `pre_cell`: A reference to the previous cell.
///
/// Returns:
/// A vector of tuples containing the winning cell and its nonce.
pub fn find_solutions<DB: DasKv, Hash, BlockNumber>(
	db: &mut DB,
	farmer_id: &FarmerId,
	pre_cell: &PreCell,
	block_hash: &Hash,
) -> Result<Vec<Solution<Hash, BlockNumber>>>
where
	BlockNumber: Clone + sp_std::hash::Hash + Encode + Decode + PartialEq,
	Hash: PartialEq + Eq + AsRef<[u8]> + Clone + 'static,
{
	let z = ZValueManager::<BlockNumber>::get_challenge(block_hash.as_ref());
	let cells = ZValueManager::get(db, z)?;

	let res = cells
		.into_iter()
		.filter_map(|(left, right)| {
			let left_cell = Piece::get_cell(&left, db).ok()?;
			let right_cell = Piece::get_cell(&right, db).ok()?;
			if let (Some(left_cell_data), Some(right_cell_data)) = (left_cell, right_cell) {
				let left_cell =
					Cell::<BlockNumber>::new(left, left_cell_data.1, &left_cell_data.0);
				let right_cell =
					Cell::<BlockNumber>::new(right, right_cell_data.1, &right_cell_data.0);
				Some(Solution::<Hash, BlockNumber>::new(
					block_hash,
					farmer_id,
					pre_cell,
					&left_cell,
					&right_cell,
				))
			} else {
				None
			}
		})
		.collect();
	Ok(res)
}

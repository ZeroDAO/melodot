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

use crate::{utils, Cell, DasKv, Decode, Encode, FarmerId, HashT, Piece, ZValueManager};
use anyhow::{Ok, Result};
use melo_das_primitives::{KZGCommitment, KZG};
use scale_info::TypeInfo;

/// `Solution` represents a potential solution in the system.
#[derive(Debug, Default, Clone, TypeInfo)]
pub struct Solution<Hash: HashT, BlockNumber>
where
	BlockNumber: Clone + sp_std::hash::Hash + Encode + Decode,
	Hash::Output: PartialEq + Eq + sp_std::hash::Hash + 'static,
{
	// The hash of the block that posted the solution.
	block_hash: Hash::Output,
	// The ID of the farmer.
	farmer_id: FarmerId,
	// The previous cell. This is the cell that in the posted block.
	pre_cell: Cell<BlockNumber>,
	// The winning cell.
	win_cell_left: Cell<BlockNumber>,
	// The winning cell.
	win_cell_right: Cell<BlockNumber>,
}

impl<Hash: HashT, BlockNumber> Solution<Hash, BlockNumber>
where
	BlockNumber: Clone + sp_std::hash::Hash + Encode + Decode + PartialEq,
{
	/// Creates a new solution.
	pub fn new(
		block_hash: &Hash::Output,
		farmer_id: &FarmerId,
		pre_cell: &Cell<BlockNumber>,
		win_cell_left: &Cell<BlockNumber>,
		win_cell_right: &Cell<BlockNumber>,
	) -> Self {
		Self {
			block_hash: block_hash.clone(),
			farmer_id: farmer_id.clone(),
			pre_cell: pre_cell.clone(),
			win_cell_left: win_cell_left.clone(),
			win_cell_right: win_cell_right.clone(),
		}
	}

	/// Verifies the correctness of the solution.
	pub fn verify(
		&self,
		pre_commit: &KZGCommitment,
		win_left_commit: &KZGCommitment,
		win_right_commit: &KZGCommitment,
		win_left_block_hash: &Hash::Output,
		win_right_block_hash: &Hash::Output,
		n: u32,
	) -> bool {
		self.validate_pre_cell(pre_commit, n) &&
			self.validate_win_cell(
				win_left_commit,
				win_right_commit,
				win_left_block_hash,
				win_right_block_hash,
				n,
			)
	}

	// Validates the previous cell.
	fn validate_pre_cell(&self, pre_commit: &KZGCommitment, n: u32) -> bool {
		let pre_cell_hash = Hash::hash_of(&self.pre_cell.data);
		let xored_hash = utils::xor_byte_slices(self.farmer_id.as_ref(), pre_cell_hash.as_ref());

		utils::is_index_valid(&xored_hash, self.pre_cell.piece_index() as usize, 32, n as usize) &&
			self.verify_kzg_proof(pre_commit, &self.pre_cell)
	}

	// Validates the winning cell.
	fn validate_win_cell(
		&self,
		win_left_commit: &KZGCommitment,
		win_right_commit: &KZGCommitment,
		win_left_block_hash: &Hash::Output,
		win_right_block_hash: &Hash::Output,
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
		) && self.verify_kzg_proof(win_left_commit, &self.win_cell_left) &&
			self.verify_kzg_proof(win_right_commit, &self.win_cell_right) &&
			ZValueManager::<BlockNumber>::verify(
				z,
				&self.farmer_id,
				&self.win_cell_left.data.into(),
				&self.win_cell_right.data.into(),
				&self.win_cell_left.metadata,
				&self.win_cell_right.metadata,
			)
	}

	// Validates the KZG proof.
	fn verify_kzg_proof(&self, commit: &KZGCommitment, cell: &Cell<BlockNumber>) -> bool {
		let kzg = KZG::default_embedded();
		kzg.verify(commit, cell.index(), &cell.data.into(), &cell.proof)
			.unwrap_or(false)
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
pub fn find_solutions<DB: DasKv, Hash: HashT, BlockNumber>(
	db: &mut DB,
	farmer_id: &FarmerId,
	pre_cell: &Cell<BlockNumber>,
	block_hash: &Hash::Output,
) -> Result<Vec<Solution<Hash, BlockNumber>>>
where
	BlockNumber: Clone + sp_std::hash::Hash + Encode + Decode + PartialEq,
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
					Cell::<BlockNumber>::new(left, left_cell_data.1.clone(), &left_cell_data.0);
				let right_cell =
					Cell::<BlockNumber>::new(right, right_cell_data.1.clone(), &right_cell_data.0);
				Some(Solution::<Hash, BlockNumber>::new(
					&block_hash,
					&farmer_id,
					&pre_cell,
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

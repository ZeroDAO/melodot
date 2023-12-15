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
	utils, BlakeTwo256, Cell, Decode, Encode, FarmerId, HashT, PreCell, Vec, ZValueManager,
};
#[cfg(feature = "std")]
use crate::{DasKv, Piece};
#[cfg(feature = "std")]
use anyhow::{Ok, Result};
use melo_das_primitives::{KZGCommitment, Segment, KZG};
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
	pub pre_cell: PreCell,
	// The winning cell.
	pub win_cell_left: Cell<BlockNumber>,
	// The winning cell.
	pub win_cell_right: Cell<BlockNumber>,
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
			farmer_id: farmer_id.clone(),
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
		let z = ZValueManager::<BlockNumber>::get_challenge(self.block_hash.as_ref());

		Self::check_pre_cell(&self.pre_cell.seg, &self.farmer_id, pre_cell_leading_zero) &&
			Self::is_index_valid(
				&self.farmer_id,
				&self.block_hash,
				self.pre_cell.piece_index() as usize,
				32,
				n,
			) && self.pre_cell.verify_kzg_proof(&kzg, pre_commit) &&
			self.validate_win_cell_with_kzg(
				&kzg,
				win_left_commit,
				win_right_commit,
				win_left_block_hash,
				win_right_block_hash,
				n,
				z,
			)
	}

	pub fn check_pre_cell(seg: &Segment, farmer_id: &FarmerId, pre_cell_leading_zero: u8) -> bool {
		let pre_cell_hash = BlakeTwo256::hash_of(seg);
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
	#[allow(clippy::too_many_arguments)]
	pub fn validate_win_cell_with_kzg(
		&self,
		kzg: &KZG,
		win_left_commit: &KZGCommitment,
		win_right_commit: &KZGCommitment,
		win_left_block_hash: &Hash,
		win_right_block_hash: &Hash,
		n: u32,
		z: u16,
	) -> bool {
		self.validate_win_cell(win_left_block_hash, win_right_block_hash, n, z) &&
			self.win_cell_left.verify_kzg_proof(kzg, win_left_commit) &&
			self.win_cell_right.verify_kzg_proof(kzg, win_right_commit)
	}

	// Validates the winning cell.
	pub fn validate_win_cell(
		&self,
		win_left_block_hash: &Hash,
		win_right_block_hash: &Hash,
		n: u32,
		z: u16,
	) -> bool {
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
		) && ZValueManager::<BlockNumber>::verify(
			z,
			&self.farmer_id,
			&self.win_cell_left.seg,
			&self.win_cell_right.seg,
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
#[cfg(feature = "std")]
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
	find_solutions_with_z(db, farmer_id, pre_cell, block_hash, z)
}

#[cfg(feature = "std")]
fn find_solutions_with_z<DB: DasKv, Hash, BlockNumber>(
	db: &mut DB,
	farmer_id: &FarmerId,
	pre_cell: &PreCell,
	block_hash: &Hash,
	z: u16,
) -> Result<Vec<Solution<Hash, BlockNumber>>>
where
	BlockNumber: Clone + sp_std::hash::Hash + Encode + Decode + PartialEq,
	Hash: PartialEq + Eq + AsRef<[u8]> + Clone + 'static,
{
	let cells = ZValueManager::get(db, z)?;

	let res = cells
		.into_iter()
		.filter_map(|(left, right)| {
			let left_cell = Piece::get_cell(&left, db).ok()?;
			let right_cell = Piece::get_cell(&right, db).ok()?;
			if let (Some(left_cell_data), Some(right_cell_data)) = (left_cell, right_cell) {
				let left_cell = Cell::<BlockNumber>::new(left, left_cell_data);
				let right_cell = Cell::<BlockNumber>::new(right, right_cell_data);

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

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{mock::*, CellMetadata, PieceMetadata, PiecePosition};
	use melo_das_db::mock_db::MockDb;
	// use melo_das_primitives::Segment;
	use sp_core::H256;

	#[test]
	fn test_is_index_valid() {
		let farmer_id = FarmerId::default();
		let block_hash: [u8; 32] = [0; 32];
		let index = 10;
		let max_index = 20;
		let n = 5;

		let result = Solution::<H256, u32>::is_index_valid(
			&farmer_id,
			&block_hash.into(),
			index,
			max_index,
			n,
		);
		assert!(!result);
	}

	#[test]
	fn test_select_indices() {
		let farmer_id = FarmerId::default();
		let block_hash: [u8; 32] = [0b00010001; 32];
		let end = 100;
		let n = 1;

		let indices = Solution::<H256, u32>::select_indices(&farmer_id, &block_hash.into(), end, n);
		assert!(!indices.is_empty());
	}

	#[test]
	fn test_find_solutions_with_z() {
		let mut db = MockDb::new();
		let farmer_id = FarmerId::default();
		let pre_cell = PreCell::default();
		let block_hash: H256 = [0; 32].into();

		let row = get_mock_row(&BLS_SCALAR11, &BLS_SCALAR12, 0, &PROOF_11, &PROOF_12, 16);

		let piece_pos = PiecePosition::Row(0);

		let piece = Piece::new(11, piece_pos, &row);

		let _ = piece.save(&mut db, &farmer_id);

		let cells = ZValueManager::<u32>::get(&mut db, Z1).unwrap();

		assert!(!cells.is_empty());

		let result = find_solutions_with_z::<MockDb, H256, u32>(
			&mut db,
			&farmer_id,
			&pre_cell,
			&block_hash,
			Z1,
		)
		.expect("Failed to find solutions");

		assert!(!result.is_empty(), "Should have found solutions");
	}

	#[test]
	fn test_solution_verify() {
		let kzg = KZG::default_embedded();
		let commitment = KZGCommitment::try_from(COMMIT1).unwrap();

		let row = get_mock_row(&BLS_SCALAR11, &BLS_SCALAR12, 0, &PROOF_11, &PROOF_12, 16);

		let pre_cell = PreCell::new(PiecePosition::Row(0), row[0].clone());

		let piece_metadata = PieceMetadata::new(5, PiecePosition::Row(0));

		let left_cell_metadata = CellMetadata::new(piece_metadata.clone(), 0);
		let right_cell_metadata = CellMetadata::new(piece_metadata, 1);

		let win_cell_left = Cell::new(left_cell_metadata, row[0].clone());

		let win_cell_right = Cell::new(right_cell_metadata, row[1].clone());

		let solution = Solution::<H256, u32>::new(
			&BLOCK_HASH1.into(),
			&FarmerId::default(),
			&pre_cell,
			&win_cell_left,
			&win_cell_right,
		);

		let check_pre =
			Solution::<H256, u32>::check_pre_cell(&solution.pre_cell.seg, &solution.farmer_id, 0);

		assert!(check_pre);

		let is_index_valid = Solution::<H256, u32>::is_index_valid(
			&solution.farmer_id,
			&solution.block_hash,
			solution.pre_cell.piece_index() as usize,
			32,
			0,
		);

		assert!(is_index_valid);

		let is_pre_cell_kzg_proof_valid = solution.pre_cell.verify_kzg_proof(&kzg, &commitment);

		assert!(is_pre_cell_kzg_proof_valid);

		let result = solution.verify(
			&commitment,
			&commitment,
			&commitment,
			&BLOCK_HASH1.into(),
			&BLOCK_HASH1.into(),
			0,
			0,
		);

		assert!(result);
	}
}

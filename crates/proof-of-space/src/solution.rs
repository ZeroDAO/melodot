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
	utils, CellMetadata, ChaCha8, DasKv, Decode, Encode, FarmerId, HashT, KZGProof, Key, KeyIvInit,
	Nonce, Position, Record, StreamCipher, XValueManager,
};
use melo_core_primitives::config::FIELD_ELEMENTS_PER_SEGMENT;
use melo_das_primitives::{KZGCommitment, SafeScalar, KZG};
use scale_info::TypeInfo;

/// `Cell` represents a unit of data in the system.
#[derive(Debug, Default, Clone, Hash, TypeInfo)]
pub struct Cell {
	// The index of the cell in the segment.
	pub index: u32,
	// The data stored in the cell.
	pub data: [u8; 31],
	// The KZG proof of the cell's data in blob.
	pub proof: KZGProof,
	// The position of the the segment.
	pub position: Position,
}

impl Cell {
	/// Calculates the absolute position of the cell based on its relative position.
	pub fn get_position_from_relative(&self) -> u32 {
		self.position.x * FIELD_ELEMENTS_PER_SEGMENT as u32 + self.index
	}
}

/// `Solution` represents a potential solution in the system.
#[derive(Debug, Default, Clone, Hash, TypeInfo)]
pub struct Solution<Hash: HashT> {
	// The hash of the block that posted the solution.
	block_hash: Hash::Output,
	// The ID of the farmer.
	farmer_id: Hash::Output,
	// The previous cell. This is the cell that in the posted block.
	pre_cell: Cell,
	// The winning cell.
	win_cell: Cell,
	// The nonce used in the solution.
	nonce: u8,
	// The hash of the winning block that win_cell is in.
	win_block_hash: Hash::Output,
}

impl<Hash: HashT> Solution<Hash> {
	/// Verifies the correctness of the solution.
	pub fn verify(
		&self,
		pre_commit: &KZGCommitment,
		win_commit: &KZGCommitment,
		n: u32,
		difficulty: u32,
	) -> bool {
		self.validate_pre_cell(pre_commit, n) &&
			self.validate_win_cell(win_commit, n) &&
			self.validate_nonce(difficulty)
	}

	// Validates the previous cell.
	fn validate_pre_cell(&self, pre_commit: &KZGCommitment, n: u32) -> bool {
		let pre_cell_hash = Hash::hash_of(&self.pre_cell.data);
		let xored_hash = utils::xor_byte_slices(self.farmer_id.as_ref(), pre_cell_hash.as_ref());

		utils::is_index_valid(&xored_hash, self.pre_cell.index as usize, 32, n as usize) &&
			self.verify_kzg_proof(pre_commit, &self.pre_cell)
	}

	// Validates the winning cell.
	fn validate_win_cell(&self, win_commit: &KZGCommitment, n: u32) -> bool {
		utils::is_index_valid(
			&self.win_block_hash.as_ref(),
			self.win_cell.index as usize,
			32,
			n as usize,
		) && self.verify_kzg_proof(win_commit, &self.win_cell)
	}

	// Validates the KZG proof.
	fn verify_kzg_proof(&self, commit: &KZGCommitment, cell: &Cell) -> bool {
		let kzg = KZG::default_embedded();
		kzg.verify(commit, cell.get_position_from_relative(), &cell.data.into(), &cell.proof)
			.unwrap_or(false)
	}

	// Validates the nonce.
	fn validate_nonce(&self, difficulty: u32) -> bool {
		let key = Key::from_slice(&self.farmer_id.as_ref());
		let binding = self.nonce.to_le_bytes();
		let nonce = Nonce::from_slice(&binding);
		let mut cipher = ChaCha8::new(&key, &nonce);
		let mut buffer = [0u8; 32];
		cipher.apply_keystream(&mut buffer);

		utils::validate_leading_zeros(&buffer, difficulty)
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
	block_num: BlockNumber,
	pre_cell: &Cell,
) -> Vec<(Cell, u8)>
where
	BlockNumber: Clone + sp_std::hash::Hash + Encode + Decode,
{
	let pre_x_manager = XValueManager::<Hash, BlockNumber>::new(
		farmer_id.clone(),
		block_num,
		Position::default(),
		pre_cell.index,
	);
	let pre_first_x = pre_x_manager.calculate_initial_x();

	pre_x_manager
		.x_values_iterator(16)
		.filter_map(|(x, _)| {
			find_matching_cells::<DB, Hash, BlockNumber>(db, farmer_id, x, pre_first_x)
		})
		.collect()
}

// Auxiliary function to find matching cells.
fn find_matching_cells<DB: DasKv, Hash: HashT, BlockNumber>(
	db: &mut DB,
	farmer_id: &FarmerId,
	x: u32,
	pre_first_x: u32,
) -> Option<(Cell, u8)>
where
	BlockNumber: Clone + sp_std::hash::Hash + Encode + Decode,
{
	let cell_key = x.to_be_bytes();
	if let Some(cell_metadata_data) = db.get(&cell_key) {
		if let Ok(cell_metadata) = CellMetadata::decode(&mut &cell_metadata_data[..]) {
			if let Some(record_data) = db.get(&cell_metadata.record_id) {
				if let Ok(record) = Record::decode(&mut &record_data[..]) {
					return find_winning_cell::<Hash, BlockNumber>(
						farmer_id,
						&record,
						cell_metadata,
						pre_first_x,
					)
				}
			}
		}
	}
	None
}

// Function to find the winning cell.
fn find_winning_cell<Hash: HashT, BlockNumber>(
	farmer_id: &FarmerId,
	record: &Record<BlockNumber>,
	cell_metadata: CellMetadata,
	pre_first_x: u32,
) -> Option<(Cell, u8)>
where
	BlockNumber: Clone + sp_std::hash::Hash + Encode + Decode,
{
	let win_x_manager = XValueManager::<Hash, BlockNumber>::new(
		farmer_id.clone(),
		record.block_num.clone(),
		record.seg.position.clone(),
		cell_metadata.index,
	);

	win_x_manager
		.x_values_iterator(16)
		.find(|&(x, _)| x == pre_first_x)
		.map(|(_, nonce)| {
			let win_cell = Cell {
				index: cell_metadata.index,
				data: record.seg.content.data[cell_metadata.index as usize].clone().to_bytes_safe(),
				proof: record.seg.content.proof.clone(),
				position: record.seg.position.clone(),
			};
			(win_cell, nonce)
		})
}

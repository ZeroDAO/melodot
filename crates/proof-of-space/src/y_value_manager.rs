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
	utils, BlsScalar, CellMetadata, ChaCha8, DasKv, Decode, Encode, FarmerId, KeyIvInit, Nonce,
	PieceMetadata, StreamCipher, Vec,
};
use alloc::vec;
#[cfg(feature = "std")]
use anyhow::{Context, Result};
use chacha20::cipher::generic_array::GenericArray;

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub enum YPos {
	Left(u32),
	Right(u32),
}

impl YPos {
	pub fn to_u32(&self) -> u32 {
		match self {
			YPos::Left(pos) => *pos,
			YPos::Right(pos) => *pos,
		}
	}

	pub fn from_u32(pos: u32) -> Self {
		if pos % 2 == 0 {
			YPos::Left(pos)
		} else {
			YPos::Right(pos)
		}
	}

	pub fn match_x_pos(&self) -> YPos {
		match self {
			YPos::Left(pos) => YPos::Right(pos + 1),
			YPos::Right(pos) => YPos::Left(pos - 1),
		}
	}

	pub fn is_pair(&self, other: &Self) -> bool {
		match self {
			YPos::Left(pos) => match other {
				YPos::Right(other_pos) => *pos + 1 == *other_pos,
				_ => false,
			},
			YPos::Right(pos) => match other {
				YPos::Left(other_pos) => *pos - 1 == *other_pos,
				_ => false,
			},
		}
	}
}

pub struct XValueManager<BlockNumber>
where
	BlockNumber: Clone + sp_std::hash::Hash + Encode + Decode,
{
	y: u32,
	cell_metadata: CellMetadata<BlockNumber>,
	pos: YPos,
}

impl<BlockNumber> XValueManager<BlockNumber>
where
	BlockNumber: Clone + sp_std::hash::Hash + Encode + Decode,
{
	pub fn calculate_y(farmer_id: &FarmerId, bls_scalar: &BlsScalar) -> u32 {
		let bytes = utils::xor_byte_slices(&farmer_id.encode(), &bls_scalar.encode());
		// Encode::encode(&(farmer_id, bls_scalar));
		let nonce = Nonce::default();
		let key = GenericArray::from_slice(&bytes);
		let mut cipher = ChaCha8::new(key, &nonce);
		let mut buffer = [0u8; 32];
		cipher.apply_keystream(&mut buffer);
		utils::fold_hash(&buffer)
	}

	pub fn new(piece_metadata: &PieceMetadata<BlockNumber>, index: u32, y: u32) -> Self {
		let pos = YPos::from_u32(index);
		Self {
			y,
			cell_metadata: CellMetadata { piece_metadata: piece_metadata.clone(), pos: index },
			pos,
		}
	}

	pub fn key(&self) -> Vec<u8> {
		Self::key_by_x_pos(&self.pos, self.y)
	}

	pub fn key_by_x_pos(y_pos: &YPos, y: u32) -> Vec<u8> {
		let mut key = Encode::encode(&y);
		key.append(&mut Encode::encode(y_pos));
		key
	}

	#[cfg(feature = "std")]
	pub fn match_cells(&self, db: &mut impl DasKv) -> Result<Vec<CellMetadata<BlockNumber>>> {
		let match_pos = self.pos.match_x_pos();
		db.get(&Self::key_by_x_pos(&match_pos, self.y))
			.map(|data| Decode::decode(&mut &data[..]))
			.transpose()
			.context("Failed to decode CellMetadata vector from database")
			.map(|opt| opt.unwrap_or_default())
	}

	#[cfg(feature = "std")]
	pub fn save(&self, db: &mut impl DasKv) {
		let key = self.key();
		match db.get(&key) {
			Some(data) => {
				let mut cell_metadatas: Vec<CellMetadata<BlockNumber>> =
					Decode::decode(&mut &data[..]).unwrap();
				cell_metadatas.push(self.cell_metadata.clone());
				db.set(&key, &cell_metadatas.encode());
			},
			None => {
				let cell_metadatas = vec![self.cell_metadata.clone()];
				db.set(&key, &cell_metadatas.encode());
			},
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{mock::*, Piece, PiecePosition};
	use melo_core_primitives::config::FIELD_ELEMENTS_PER_SEGMENT;
	use melo_das_db::mock_db::MockDb;
	use melo_das_primitives::{Position, Segment};
	use rand::{thread_rng, Rng};

	fn random_bls_scalar() -> BlsScalar {
		let mut rng = thread_rng();
		let mut bytes = [0u8; 31];
		rng.fill(&mut bytes[..]);

		BlsScalar::from(&bytes)
	}

	#[test]
	fn test_ypos_left_to_u32() {
		let ypos = YPos::Left(10);
		assert_eq!(ypos.to_u32(), 10);
	}

	#[test]
	fn test_ypos_right_to_u32() {
		let ypos = YPos::Right(15);
		assert_eq!(ypos.to_u32(), 15);
	}

	#[test]
	fn test_ypos_from_u32_even() {
		let ypos = YPos::from_u32(8);
		assert_eq!(ypos, YPos::Left(8));
	}

	#[test]
	fn test_ypos_from_u32_odd() {
		let ypos = YPos::from_u32(7);
		assert_eq!(ypos, YPos::Right(7));
	}

	#[test]
	fn test_ypos_match_x_pos_left_to_right() {
		let ypos = YPos::Left(2);
		assert_eq!(ypos.match_x_pos(), YPos::Right(3));
	}

	#[test]
	fn test_ypos_match_x_pos_right_to_left() {
		let ypos = YPos::Right(3);
		assert_eq!(ypos.match_x_pos(), YPos::Left(2));
	}

	#[test]
	fn test_ypos_is_pair_correct() {
		let ypos1 = YPos::Left(2);
		let ypos2 = YPos::Right(3);
		assert!(ypos1.is_pair(&ypos2));
	}

	#[test]
	fn test_ypos_is_pair_incorrect() {
		let ypos1 = YPos::Left(2);
		let ypos2 = YPos::Right(4);
		assert!(!ypos1.is_pair(&ypos2));
	}

	#[test]
	fn test_ypos_to_and_from_u32() {
		let pos = 5;
		let ypos = YPos::from_u32(pos);
		assert_eq!(ypos, YPos::Right(pos));
		assert_eq!(ypos.to_u32(), pos);
	}

	#[test]
	fn test_ypos_match_x_pos() {
		let ypos_left = YPos::Left(2);
		let ypos_right = ypos_left.match_x_pos();
		assert_eq!(ypos_right, YPos::Right(3));
	}

	#[test]
	fn test_ypos_is_pair() {
		let ypos_left = YPos::Left(2);
		let ypos_right = YPos::Right(3);
		assert!(ypos_left.is_pair(&ypos_right));
		assert!(ypos_right.is_pair(&ypos_left));

		let ypos_right = YPos::Left(4);
		assert!(!ypos_left.is_pair(&ypos_right));
	}

	#[test]
	fn test_x_value_manager_new() {
		let piece_metadata = PieceMetadata::<u32>::default();
		let x_value_manager = XValueManager::new(&piece_metadata, 10, 20);
		assert_eq!(x_value_manager.y, 20);
		assert_eq!(x_value_manager.pos, YPos::from_u32(10));
	}

	#[test]
	fn test_x_value_manager_key() {
		let piece_metadata = PieceMetadata::<u32>::default();
		let x_value_manager = XValueManager::new(&piece_metadata, 10, 20);
		let key = x_value_manager.key();
		let expected_key = XValueManager::<u32>::key_by_x_pos(&YPos::from_u32(10), 20);
		assert_eq!(key, expected_key);
	}

	fn test_calculate_y_case(bs: &[u8; 31], y_e: u32) {
		let farmer_id = FarmerId::default();
		let bls_scalar = BlsScalar::from(bs);

		let y = XValueManager::<u32>::calculate_y(&farmer_id, &bls_scalar);

		assert!(y == y_e);
	}

	#[test]
	fn test_calculate_y() {
		test_calculate_y_case(&BLS_SCALAR11, Y1);
		test_calculate_y_case(&BLS_SCALAR12, Y1);
		test_calculate_y_case(&BLS_SCALAR21, Y2);
		test_calculate_y_case(&BLS_SCALAR22, Y2);
		test_calculate_y_case(&BLS_SCALAR31, Y3);
		test_calculate_y_case(&BLS_SCALAR32, Y3);
	}

	#[test]
	fn test_x_value_manager_new_and_key() {
		let piece_metadata = PieceMetadata::<u32>::default();
		let index = 5;
		let y = 10;

		let x_value_manager = XValueManager::new(&piece_metadata, index, y);

		assert_eq!(x_value_manager.pos, YPos::from_u32(index));

		let key = x_value_manager.key();
		let expected_key = XValueManager::<u32>::key_by_x_pos(&YPos::from_u32(index), y);
		assert_eq!(key, expected_key);
	}

	fn mock_piece_store(b: &[u8; 31], x: u32, y: u32, index: u32, is_row: bool, db: &mut MockDb) {
		let bls_scalar = BlsScalar::from(b);
		let farmer_id = FarmerId::default();

		let bls_rand = random_bls_scalar();

		let position = Position { x, y };
		let piece_position = if is_row {
			PiecePosition::from_row(&position)
		} else {
			PiecePosition::from_column(&position)
		};

		let mut segment = Segment::default();
		segment.position = position;
		segment.content.data = [bls_rand; FIELD_ELEMENTS_PER_SEGMENT].to_vec();
		segment.content.data[index as usize] = bls_scalar;

		let piece = Piece::new(11, piece_position, &[segment]);

		let _ = piece.save(db, &farmer_id);
	}

	#[test]
	fn test_x_value_manager_match_cells() {
		let mut db = MockDb::new();

		mock_piece_store(&BLS_SCALAR11, 0, 1, 0, true, &mut db);
		mock_piece_store(&BLS_SCALAR12, 0, 1, 1, true, &mut db);

		mock_piece_store(&BLS_SCALAR21, 2, 2, 0, true, &mut db);
		mock_piece_store(&BLS_SCALAR22, 2, 2, 1, true, &mut db);

		mock_piece_store(&BLS_SCALAR31, 4, 3, 2, true, &mut db);
		mock_piece_store(&BLS_SCALAR32, 9, 3, 3, true, &mut db);

		let x_value_manager = XValueManager::new(&PieceMetadata::<u32>::default(), 0, Y1);
		let match_cells_set = x_value_manager.match_cells(&mut db).unwrap();

		assert_eq!(match_cells_set.len(), 1);

		let x_value_manager = XValueManager::new(&PieceMetadata::<u32>::default(), 0, Y2);
		let match_cells_set = x_value_manager.match_cells(&mut db).unwrap();

		assert_eq!(match_cells_set.len(), 1);

		let x_value_manager = XValueManager::new(&PieceMetadata::<u32>::default(), 0, Y3);
		let match_cells_set = x_value_manager.match_cells(&mut db).unwrap();

		assert_eq!(match_cells_set.len(), 0);

		mock_piece_store(&BLS_SCALAR12, 0, 0, 1, false, &mut db);

		let x_value_manager = XValueManager::new(&PieceMetadata::<u32>::default(), 0, Y1);
		let match_cells_set = x_value_manager.match_cells(&mut db).unwrap();

		assert_eq!(match_cells_set.len(), 2);
	}

	#[test]
	fn test_x_value_manager_save() {
		let mut db = MockDb::new();
		let piece_metadata = PieceMetadata::<u32>::default();
		let index = 5;
		let y = 10;

		let x_value_manager = XValueManager::new(&piece_metadata, index, y);

		x_value_manager.save(&mut db);

		let key = x_value_manager.key();
		let saved_data = db.get(&key).expect("Data should be saved");
		let cell_metadatas: Vec<CellMetadata<u32>> = Decode::decode(&mut &saved_data[..]).unwrap();
		assert!(!cell_metadatas.is_empty());
		assert_eq!(cell_metadatas[0], x_value_manager.cell_metadata);
	}
}

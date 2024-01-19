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

#[cfg(feature = "std")]
use crate::DasKv;
use crate::{
	utils, CellMetadata, ChaCha8, Decode, Encode, FarmerId, KeyIvInit, Nonce, PieceMetadata,
	StreamCipher, Vec,
};
#[cfg(feature = "std")]
use anyhow::{Context, Result};
use chacha20::cipher::generic_array::GenericArray;
use melo_das_primitives::Segment;

/// Represents the Y-position in a 2D space, with options for Left and Right positions.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub enum YPos {
	Left(u32),
	Right(u32),
}

impl YPos {

	/// Converts `YPos` to its `u32` representation.
	pub fn to_u32(&self) -> u32 {
		match self {
			YPos::Left(pos) => *pos,
			YPos::Right(pos) => *pos,
		}
	}

    /// Creates a `YPos` from a `u32`, determining Left or Right based on parity.
	pub fn from_u32(pos: u32) -> Self {
		if pos % 2 == 0 {
			YPos::Left(pos)
		} else {
			YPos::Right(pos)
		}
	}

    /// Matches `YPos` to its counterpart (Left to Right and vice versa) with an offset.
	pub fn match_x_pos(&self) -> YPos {
		match self {
			YPos::Left(pos) => YPos::Right(pos + 1),
			YPos::Right(pos) => YPos::Left(pos - 1),
		}
	}

    /// Determines if two `YPos` instances are pairs (adjacent positions).
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

/// Manages the X-value in a blockchain context, linking to `CellMetadata`.
pub struct YValueManager<BlockNumber>
where
	BlockNumber: Clone + sp_std::hash::Hash + Encode + Decode,
{
	y: u32,
	cell_metadata: CellMetadata<BlockNumber>,
	pos: YPos,
}

impl<BlockNumber> YValueManager<BlockNumber>
where
	BlockNumber: Clone + sp_std::hash::Hash + Encode + Decode,
{
	/// Calculates the Y-value based on `FarmerId` and a `Segment`.
	pub fn calculate_y(farmer_id: &FarmerId, seg: &Segment) -> u32 {
		if seg.content.data.is_empty() {
			return 0u32
		}
		let bytes = utils::xor_byte_slices(&farmer_id.encode(), &seg.content.data[0].encode());
		let nonce = Nonce::default();
		let key = GenericArray::from_slice(&bytes);
		let mut cipher = ChaCha8::new(key, &nonce);
		let mut buffer = [0u8; 32];
		cipher.apply_keystream(&mut buffer);
		utils::fold_hash(&buffer)
	}

    /// Constructs a new `YValueManager` instance.
	pub fn new(piece_metadata: &PieceMetadata<BlockNumber>, index: u32, y: u32) -> Self {
		let pos = YPos::from_u32(index);
		Self {
			y,
			cell_metadata: CellMetadata { piece_metadata: piece_metadata.clone(), offset: index },
			pos,
		}
	}

    /// Generates a key for the current `YValueManager`.
	pub fn key(&self) -> Vec<u8> {
		Self::key_by_x_pos(&self.pos, self.y)
	}

    /// Generates a key based on `YPos` and a Y-value.
	pub fn key_by_x_pos(y_pos: &YPos, y: u32) -> Vec<u8> {
		let mut key = Encode::encode(&y);
		key.append(&mut Encode::encode(y_pos));
		key
	}

    /// Conditionally compiled method to match cells in a database using `DasKv`.
	#[cfg(feature = "std")]
	pub fn match_cells(&self, db: &mut impl DasKv) -> Result<Vec<CellMetadata<BlockNumber>>> {
		let match_pos = self.pos.match_x_pos();
		db.get(&Self::key_by_x_pos(&match_pos, self.y))
			.map(|data| Decode::decode(&mut &data[..]))
			.transpose()
			.context("Failed to decode CellMetadata vector from database")
			.map(|opt| opt.unwrap_or_default())
	}

    /// Conditionally compiled method to save cell metadata to a database.
	#[cfg(feature = "std")]
	pub fn save(&self, db: &mut impl DasKv) {
		if self.y == 0 {
			return
		}
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
	use melo_das_primitives::Position;
	#[cfg(feature = "special_tests")]
	use rand::{thread_rng, Rng};

	#[cfg(feature = "special_tests")]
	fn random_bls_scalar() -> [u8; 31] {
		let mut rng = thread_rng();
		let mut bytes = [0u8; 31];
		rng.fill(&mut bytes[..]);

		bytes
	}

	#[cfg(feature = "special_tests")]
	fn mock_y(b1: &[u8; 31], b2: &[u8; 31]) -> bool {
		let chunk_len: usize = 16;
		let chunk_count: usize = SEGMENTS_PER_BLOB;
		let num_shards = chunk_len * chunk_count;

		let kzg = KZG::default_embedded();

		let mut row = [None; SEGMENTS_PER_BLOB * 16 * 2];
		for i in 0..num_shards {
			row[i] = Some(BlsScalar::from([1u8; 31]));
		}

		row[0] = Some(BlsScalar::from(b1));
		row[chunk_len] = Some(BlsScalar::from(b2));

		let poly = recover_poly(kzg.get_fs(), &row).unwrap();

		let commitment = kzg.commit(&poly).unwrap();
		let all_proofs = kzg.all_proofs(&poly, chunk_len).unwrap();

		let segs = poly_to_segment_vec(&poly, &kzg, 0, chunk_len).unwrap();

		let y1 = YValueManager::<u32>::calculate_y(&FarmerId::default(), &segs[0]);
		let y2 = YValueManager::<u32>::calculate_y(&FarmerId::default(), &segs[1]);

		if y1 == y2 {
			print!("y: {:?}\n", y1);
			print!("bytes1: {:?}\n", b1);
			print!("bytes2: {:?}\n", b2);
			print!("commitment: {:?}\n", commitment.to_bytes());
			print!("proof_1: {:?}\n", all_proofs[0].to_bytes());
			print!("proof_2: {:?}\n", all_proofs[1].to_bytes());
		}

		false
	}

	#[cfg(feature = "special_tests")]
	#[test]
	fn find_equal_y() {
		mock_y(&BLS_SCALAR11, &BLS_SCALAR12);
		mock_y(&BLS_SCALAR21, &BLS_SCALAR22);
		mock_y(&BLS_SCALAR31, &BLS_SCALAR32);
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
		let x_value_manager = YValueManager::new(&piece_metadata, 10, 20);
		assert_eq!(x_value_manager.y, 20);
		assert_eq!(x_value_manager.pos, YPos::from_u32(10));
	}

	#[test]
	fn test_x_value_manager_key() {
		let piece_metadata = PieceMetadata::<u32>::default();
		let x_value_manager = YValueManager::new(&piece_metadata, 10, 20);
		let key = x_value_manager.key();
		let expected_key = YValueManager::<u32>::key_by_x_pos(&YPos::from_u32(10), 20);
		assert_eq!(key, expected_key);
	}

	fn test_calculate_y_case(bs: &[u8; 31], proof: &[u8; 48], y_e: u32) {
		let farmer_id = FarmerId::default();

		let seg = get_mock_seg(bs, 0, 0, &proof, FIELD_ELEMENTS_PER_SEGMENT);

		let y = YValueManager::<u32>::calculate_y(&farmer_id, &seg);

		assert!(y == y_e);
	}

	#[test]
	fn test_calculate_y() {
		test_calculate_y_case(&BLS_SCALAR11, &PROOF_11, Y1);
		test_calculate_y_case(&BLS_SCALAR12, &PROOF_12, Y1);
		test_calculate_y_case(&BLS_SCALAR21, &PROOF_21, Y2);
		test_calculate_y_case(&BLS_SCALAR22, &PROOF_22, Y2);
		test_calculate_y_case(&BLS_SCALAR31, &PROOF_31, Y3);
		test_calculate_y_case(&BLS_SCALAR32, &PROOF_32, Y3);
	}

	#[test]
	fn test_x_value_manager_new_and_key() {
		let piece_metadata = PieceMetadata::<u32>::default();
		let index = 5;
		let y = 10;

		let x_value_manager = YValueManager::new(&piece_metadata, index, y);

		assert_eq!(x_value_manager.pos, YPos::from_u32(index));

		let key = x_value_manager.key();
		let expected_key = YValueManager::<u32>::key_by_x_pos(&YPos::from_u32(index), y);
		assert_eq!(key, expected_key);
	}

	fn mock_piece_store(
		block_num: u32,
		b1: &[u8; 31],
		b2: &[u8; 31],
		proof1: &[u8; 48],
		proof2: &[u8; 48],
		x: u32,
		y: u32,
		is_row: bool,
		db: &mut MockDb,
	) {
		let farmer_id = FarmerId::default();

		let row = get_mock_row(b1, b2, y, &proof1, &proof2, FIELD_ELEMENTS_PER_SEGMENT);

		let position = Position { x, y };

		let piece_position = if is_row {
			PiecePosition::from_row(&position)
		} else {
			PiecePosition::from_column(&position)
		};

		let piece = Piece::new(block_num, piece_position, &row);

		let _ = piece.save(db, &farmer_id);
	}

	#[test]
	fn test_x_value_manager_match_cells() {
		let mut db = MockDb::new();

		mock_piece_store(
			1,
			&BLS_SCALAR11,
			&BLS_SCALAR12,
			&PROOF_11,
			&PROOF_12,
			0,
			0,
			true,
			&mut db,
		);
		mock_piece_store(
			2,
			&BLS_SCALAR21,
			&BLS_SCALAR22,
			&PROOF_21,
			&PROOF_22,
			0,
			0,
			true,
			&mut db,
		);

		let x_value_manager = YValueManager::new(&PieceMetadata::<u32>::default(), 0, Y1);
		let match_cells_set = x_value_manager.match_cells(&mut db).unwrap();

		assert_eq!(match_cells_set.len(), 1);
	}

	#[test]
	fn test_x_value_manager_save() {
		let mut db = MockDb::new();
		let piece_metadata = PieceMetadata::<u32>::default();
		let index = 5;
		let y = 10;

		let x_value_manager = YValueManager::new(&piece_metadata, index, y);

		x_value_manager.save(&mut db);

		let key = x_value_manager.key();
		let saved_data = db.get(&key).expect("Data should be saved");
		let cell_metadatas: Vec<CellMetadata<u32>> = Decode::decode(&mut &saved_data[..]).unwrap();
		assert!(!cell_metadatas.is_empty());
		assert_eq!(cell_metadatas[0], x_value_manager.cell_metadata);
	}
}

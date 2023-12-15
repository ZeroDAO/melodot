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
use crate::{utils, BlakeTwo256, CellMetadata, Decode, Encode, FarmerId, HashT, XValueManager};
#[cfg(feature = "std")]
use anyhow::{Context, Result};
use melo_das_primitives::Segment;

/// Represents a manager for Z values in a specific blockchain context.
/// Z values are calculated based on left and right cell data and used for
/// various verification and management tasks within the blockchain system.
///
/// `BlockNumber` is a generic parameter that represents the type of the block number in the
/// blockchain.
#[derive(Default, Clone, PartialEq, Eq, Encode, Decode)]
pub struct ZValueManager<BlockNumber>
where
	BlockNumber: Clone + sp_std::hash::Hash + Encode + Decode + PartialEq,
{
	z: u16,
	left: CellMetadata<BlockNumber>,
	right: CellMetadata<BlockNumber>,
}

impl<BlockNumber> ZValueManager<BlockNumber>
where
	BlockNumber: Clone + sp_std::hash::Hash + Encode + Decode + PartialEq,
{
	/// Calculates the Z value from given left and right segments of the blockchain.
	/// This involves encoding the segments, combining them, and then hashing the combination
	/// to produce a final Z value.
	///
	/// - `left_cell`: The left segment of the blockchain.
	/// - `right_cell`: The right segment of the blockchain.
	pub fn calculate_z(left_cell: &Segment, right_cell: &Segment) -> u16 {
		let left_encoded = Encode::encode(left_cell);
		let right_encoded = Encode::encode(right_cell);

		let combined = [left_encoded, right_encoded].concat();

		let hash = BlakeTwo256::hash(&combined);
		utils::hash_to_u16_xor(hash.as_bytes())
	}

	/// Generates a challenge value (Z) directly from raw data.
	/// This is typically used in scenarios where raw data needs to be quickly converted
	/// into a Z value without the need for segment processing.
	///
	/// - `data`: Raw data used to calculate the challenge value.
	pub fn get_challenge(data: &[u8]) -> u16 {
		utils::hash_to_u16_xor(data)
	}

	/// Retrieves the stored Z value of the current instance.
	pub fn get_z(&self) -> u16 {
		self.z
	}

	/// Creates a new instance of `ZValueManager` using the provided cell metadata and segments.
	///
	/// - `left`: Metadata for the left cell.
	/// - `right`: Metadata for the right cell.
	/// - `left_cell`: The left segment of the blockchain.
	/// - `right_cell`: The right segment of the blockchain.
	pub fn new(
		left: &CellMetadata<BlockNumber>,
		right: &CellMetadata<BlockNumber>,
		left_cell: &Segment,
		right_cell: &Segment,
	) -> Self {
		let z = Self::calculate_z(left_cell, right_cell);
		Self { z, left: left.clone(), right: right.clone() }
	}

	/// Saves the current state of the `ZValueManager` to a database.
	/// This includes the Z value and associated cell metadata.
	/// Only available when compiled with the `std` feature.
	///
	/// - `db`: A mutable reference to the database where the data will be stored.
	#[cfg(feature = "std")]
	pub fn save(&self, db: &mut impl DasKv) {
		let key = Encode::encode(&self.z);
		let existing_data = db.get(&key);

		let mut pairs: Vec<(CellMetadata<BlockNumber>, CellMetadata<BlockNumber>)> = existing_data
			.map(|data| Decode::decode(&mut &data[..]).unwrap_or_default())
			.unwrap_or_default();

		let pair = (self.left.clone(), self.right.clone());
		if !pairs.contains(&pair) {
			pairs.push(pair);
			db.set(&key, &Encode::encode(&pairs));
		}
	}

	/// Retrieves a list of cell metadata pairs associated with a given Z value from the database.
	/// Only available when compiled with the `std` feature.
	///
	/// - `db`: A mutable reference to the database to query.
	/// - `z`: The Z value for which to retrieve cell metadata pairs.
	#[cfg(feature = "std")]
	pub fn get(
		db: &mut impl DasKv,
		z: u16,
	) -> Result<Vec<(CellMetadata<BlockNumber>, CellMetadata<BlockNumber>)>> {
		let key = Encode::encode(&z);
		db.get(&key)
			.map(|data| Decode::decode(&mut &data[..]))
			.transpose()
			.context("Failed to decode CellMetadata Vec from database")
			.map(|opt| opt.unwrap_or_default())
	}

	/// Verifies whether the given Z value, farmer ID, and cell segments and metadata
	/// match the expected criteria for validation.
	///
	/// - `z`: The Z value to verify.
	/// - `farmer_id`: The ID of the farmer.
	/// - `left_cell`: The left segment of the blockchain.
	/// - `right_cell`: The right segment of the blockchain.
	/// - `left_cell_metadata`: Metadata for the left cell.
	/// - `right_cell_metadata`: Metadata for the right cell.
	pub fn verify(
		z: u16,
		farmer_id: &FarmerId,
		left_cell: &Segment,
		right_cell: &Segment,
		left_cell_metadata: &CellMetadata<BlockNumber>,
		right_cell_metadata: &CellMetadata<BlockNumber>,
	) -> bool {
		let calculated_z = Self::calculate_z(left_cell, right_cell);
		let is_pair = left_cell_metadata.is_pair(right_cell_metadata);
		let is_y_equal = XValueManager::<BlockNumber>::calculate_y(farmer_id, left_cell) ==
			XValueManager::<BlockNumber>::calculate_y(farmer_id, right_cell);
		z == calculated_z && is_pair && is_y_equal
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{mock::*, PieceMetadata, PiecePosition};
	use melo_das_db::mock_db::MockDb;

	fn test_calculate_z_case(
		left_cell: &[u8; 31],
		right_cell: &[u8; 31],
		left_proof: &[u8; 48],
		right_proof: &[u8; 48],
		expected_z: u16,
	) {
		let left_seg = get_mock_seg(left_cell, 0, 0, &left_proof, 16);
		let right_seg = get_mock_seg(right_cell, 1, 0, &right_proof, 16);

		let z = ZValueManager::<u16>::calculate_z(&left_seg, &right_seg);
		assert_eq!(z, expected_z);
	}

	#[test]
	fn test_calculate_z() {
		test_calculate_z_case(&BLS_SCALAR11, &BLS_SCALAR12, &PROOF_11, &PROOF_12, Z1);
		test_calculate_z_case(&BLS_SCALAR21, &BLS_SCALAR22, &PROOF_21, &PROOF_22, Z2);
		test_calculate_z_case(&BLS_SCALAR31, &BLS_SCALAR32, &PROOF_31, &PROOF_32, Z3);
	}

	fn z_store(
		left_cell: &[u8; 31],
		right_cell: &[u8; 31],
		left_proof: &[u8; 48],
		right_proof: &[u8; 48],
		expected_z: u16,
		db: &mut impl DasKv,
	) {
		let left_seg = get_mock_seg(left_cell, 0, 0, &left_proof, 16);
		let right_seg = get_mock_seg(right_cell, 1, 0, &right_proof, 16);

		let left_metadata = CellMetadata::<u16>::default();
		let right_metadata = CellMetadata::<u16>::default();

		let zvm = ZValueManager::new(&left_metadata, &right_metadata, &left_seg, &right_seg);
		assert_eq!(zvm.z, expected_z);
		zvm.save(db);
	}

	#[test]
	fn test_get() {
		let mut db = MockDb::new();
		z_store(&BLS_SCALAR11, &BLS_SCALAR12, &PROOF_11, &PROOF_12, Z1, &mut db);
		let zvms = ZValueManager::<u16>::get(&mut db, Z1).unwrap();
		assert_eq!(zvms.len(), 1);
		z_store(&BLS_SCALAR21, &BLS_SCALAR22, &PROOF_21, &PROOF_22, Z2, &mut db);
		let zvms = ZValueManager::<u16>::get(&mut db, Z2).unwrap();
		assert_eq!(zvms.len(), 1);

		let zvms = ZValueManager::<u16>::get(&mut db, Z3).unwrap();
		assert_eq!(zvms.len(), 0);

		z_store(&BLS_SCALAR31, &BLS_SCALAR32, &PROOF_31, &PROOF_32, Z3, &mut db);
		let zvms = ZValueManager::<u16>::get(&mut db, Z3).unwrap();
		assert_eq!(zvms.len(), 1);

		let zvms = ZValueManager::<u16>::get(&mut db, 123).unwrap();
		assert_eq!(zvms.len(), 0);
	}

	#[test]
	fn test_verify() {
		let farmer_id = FarmerId::default();

		let position_left = PiecePosition::Row(0);
		let position_right = PiecePosition::Row(0);

		let left_seg = get_mock_seg(&BLS_SCALAR11, 0, 0, &PROOF_11, 16);
		let right_seg = get_mock_seg(&BLS_SCALAR12, 1, 0, &PROOF_12, 16);

		let piece_metadata_left = PieceMetadata::<u32>::new(2, position_left);
		let piece_metadata_right = PieceMetadata::<u32>::new(1, position_right);

		let left_metadata = CellMetadata::<u32>::new(piece_metadata_left, 0);
		let right_metadata = CellMetadata::<u32>::new(piece_metadata_right, 1);

		let is_valid = ZValueManager::<u32>::verify(
			123,
			&farmer_id,
			&left_seg,
			&right_seg,
			&left_metadata,
			&right_metadata,
		);
		assert!(!is_valid);

		let is_valid = ZValueManager::<u32>::verify(
			u16::MAX,
			&farmer_id,
			&left_seg,
			&right_seg,
			&left_metadata,
			&right_metadata,
		);
		assert!(!is_valid);

		let is_valid = ZValueManager::<u32>::verify(
			Z1,
			&farmer_id,
			&left_seg,
			&right_seg,
			&left_metadata,
			&right_metadata,
		);

		assert!(is_valid);
	}
}

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
	utils, BlakeTwo256, BlsScalar, CellMetadata, DasKv, Decode, Encode, FarmerId, HashT, Vec,
	XValueManager,
};
#[cfg(feature = "std")]
use anyhow::{Context, Result};

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
	pub fn calculate_z(left_cell: &BlsScalar, right_cell: &BlsScalar) -> u16 {
		let left_encoded = Encode::encode(left_cell);
		let right_encoded = Encode::encode(right_cell);

		let combined = [left_encoded, right_encoded].concat();

		let hash = BlakeTwo256::hash(&combined);
		utils::hash_to_u16_xor(hash.as_bytes())
	}

	pub fn get_challenge(data: &[u8]) -> u16 {
		utils::hash_to_u16_xor(data)
	}

	pub fn get_z(&self) -> u16 {
		self.z
	}

	pub fn new(
		left: &CellMetadata<BlockNumber>,
		right: &CellMetadata<BlockNumber>,
		left_cell: &BlsScalar,
		right_cell: &BlsScalar,
	) -> Self {
		let z = Self::calculate_z(left_cell, right_cell);
		Self { z, left: left.clone(), right: right.clone() }
	}

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

	pub fn verify(
		z: u16,
		farmer_id: &FarmerId,
		left_cell: &BlsScalar,
		right_cell: &BlsScalar,
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
	use crate::{mock::*, BlsScalar, PiecePosition, PieceMetadata};
	use melo_das_db::mock_db::MockDb;
	
	fn test_calculate_z_case(
		left_cell: &[u8;31],
		right_cell: &[u8;31],
		expected_z: u16,
	) {
		let left_cell = BlsScalar::from(left_cell);
		let right_cell = BlsScalar::from(right_cell);

		let z = ZValueManager::<u16>::calculate_z(&left_cell, &right_cell);
		assert_eq!(z, expected_z);
	}

	#[test]
	fn test_calculate_z() {
		test_calculate_z_case(&BLS_SCALAR11, &BLS_SCALAR12, Z1);
		test_calculate_z_case(&BLS_SCALAR21, &BLS_SCALAR22, Z2);
		test_calculate_z_case(&BLS_SCALAR31, &BLS_SCALAR32, Z3);
	}

	fn z_store(
		left_cell: &[u8;31],
		right_cell: &[u8;31],
		expected_z: u16,
		db: &mut impl DasKv,
	) {
		let left_cell = BlsScalar::from(left_cell);
		let right_cell = BlsScalar::from(right_cell);

		let left_metadata = CellMetadata::<u16>::default();
		let right_metadata = CellMetadata::<u16>::default();

		let zvm = ZValueManager::new(&left_metadata, &right_metadata, &left_cell, &right_cell);
		assert_eq!(zvm.z, expected_z);
		zvm.save(db);
	}

	#[test]
	fn test_get() {
		let mut db = MockDb::new();
		z_store(&BLS_SCALAR11, &BLS_SCALAR12, Z1, &mut db);
		let zvms = ZValueManager::<u16>::get(&mut db, Z1).unwrap();
		assert_eq!(zvms.len(), 1);
		z_store(&BLS_SCALAR21, &BLS_SCALAR22, Z2, &mut db);
		let zvms = ZValueManager::<u16>::get(&mut db, Z2).unwrap();
		assert_eq!(zvms.len(), 1);

		let zvms = ZValueManager::<u16>::get(&mut db, Z3).unwrap();
		assert_eq!(zvms.len(), 0);

		z_store(&BLS_SCALAR31, &BLS_SCALAR32, Z3, &mut db);
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

		let left_cell = BlsScalar::from(BLS_SCALAR11);
		let right_cell = BlsScalar::from(BLS_SCALAR12);

		let piece_metadata_left = PieceMetadata::<u32>::new(2, position_left);
		let piece_metadata_right = PieceMetadata::<u32>::new(1, position_right);

		let left_metadata = CellMetadata::<u32>::new(piece_metadata_left, 0);
		let right_metadata = CellMetadata::<u32>::new(piece_metadata_right,1);

		let is_valid = ZValueManager::<u32>::verify(
			123,
			&farmer_id,
			&left_cell,
			&right_cell,
			&left_metadata,
			&right_metadata,
		);
		assert!(!is_valid);

		let is_valid = ZValueManager::<u32>::verify(
			u16::MAX,
			&farmer_id,
			&left_cell,
			&right_cell,
			&left_metadata,
			&right_metadata,
		);
		assert!(!is_valid);

		let is_valid = ZValueManager::<u32>::verify(
			Z1,
			&farmer_id,
			&left_cell,
			&right_cell,
			&left_metadata,
			&right_metadata,
		);

		assert!(is_valid);

		let left_cell = BlsScalar::from(BLS_SCALAR21);

		let is_valid = ZValueManager::<u32>::verify(
			Z1,
			&farmer_id,
			&left_cell,
			&right_cell,
			&left_metadata,
			&right_metadata,
		);

		assert!(!is_valid);
	}
}

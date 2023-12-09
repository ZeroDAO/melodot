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
	utils, BlakeTwo256, BlsScalar, CellMetadata, DasKv, Decode, Encode, FarmerId, HashT,
	XValueManager,
};
use anyhow::{Context, Result};

#[derive(Default, Clone, PartialEq, Eq, Encode, Decode)]
pub struct ZValueManager<BlockNumber>
where
	BlockNumber: Clone + sp_std::hash::Hash + Encode + Decode + PartialEq,
{
	z: u32,
	left: CellMetadata<BlockNumber>,
	right: CellMetadata<BlockNumber>,
}

impl<BlockNumber> ZValueManager<BlockNumber>
where
	BlockNumber: Clone + sp_std::hash::Hash + Encode + Decode + PartialEq,
{
	pub fn calculate_z(left_cell: &BlsScalar, right_cell: &BlsScalar) -> u32 {
		let left_encoded = Encode::encode(left_cell);
		let right_encoded = Encode::encode(right_cell);

		let combined = [left_encoded, right_encoded].concat();

		let hash = BlakeTwo256::hash(&combined);
		utils::fold_hash(hash.as_bytes())
	}

	pub fn get_challenge(data: &[u8]) -> u32 {
		utils::fold_hash(data)
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

	pub fn get(
		db: &mut impl DasKv,
		z: u32,
	) -> Result<Vec<(CellMetadata<BlockNumber>, CellMetadata<BlockNumber>)>> {
		let key = Encode::encode(&z);
		db.get(&key)
			.map(|data| Decode::decode(&mut &data[..]))
			.transpose()
			.context("Failed to decode CellMetadata Vec from database")
			.map(|opt| opt.unwrap_or_default())
	}

	pub fn verify(
		z: u32,
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

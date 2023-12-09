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
	PieceMetadata, StreamCipher,
};
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
		let bytes = Encode::encode(&(farmer_id, bls_scalar));
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

	pub fn match_cells(&self, db: &mut impl DasKv) -> Result<Vec<CellMetadata<BlockNumber>>> {
		db.get(&Self::key_by_x_pos(&self.pos, self.y))
			.map(|data| Decode::decode(&mut &data[..]))
			.transpose()
			.context("Failed to decode CellMetadata vector from database")
			.map(|opt| opt.unwrap_or_default())
	}

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

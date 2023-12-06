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

use melo_das_primitives::SafeScalar;
use scale_info::TypeInfo;
use sp_core::RuntimeDebug;
use crate::{
	BlsScalar, Decode, Encode, KZGProof, PieceMetadata, PiecePosition, Position, YPos,
	FIELD_ELEMENTS_PER_SEGMENT,
};

#[derive(Default, Clone, Debug, PartialEq, Eq, Encode, Decode, TypeInfo)]
pub struct CellMetadata<BlockNumber>
where
	BlockNumber: Clone + sp_std::hash::Hash + Encode + Decode,
{
	pub piece_metadata: PieceMetadata<BlockNumber>,
	pub pos: u32,
}

impl<BlockNumber> CellMetadata<BlockNumber>
where
	BlockNumber: Clone + sp_std::hash::Hash + Encode + Decode,
{
	pub fn seg_position(&self) -> Position {
		let index = self.pos / FIELD_ELEMENTS_PER_SEGMENT as u32;
		match self.piece_metadata.pos {
			PiecePosition::Row(_) => Position { x: self.pos, y: index },
			PiecePosition::Column(_) => Position { x: index, y: self.pos },
		}
	}

	pub fn index(&self) -> u32 {
		match self.piece_metadata.pos {
			PiecePosition::Row(_) => self.pos,
			PiecePosition::Column(_) => self.piece_metadata.pos.to_u32(),
		}
	}

	pub fn is_pair(&self, other: &Self) -> bool {
		let x_pos = YPos::from_u32(self.pos);
		let other_x_pos = YPos::from_u32(other.pos);
		x_pos.is_pair(&other_x_pos)
	}

	pub fn block_number(&self) -> BlockNumber {
		self.piece_metadata.block_num.clone()
	}
}

/// `Cell` represents a unit of data in the system.
#[derive(Encode, Decode, RuntimeDebug, Default, Clone, PartialEq, Eq, TypeInfo)]
pub struct Cell<BlockNumber>
where
	BlockNumber: Clone + sp_std::hash::Hash + Encode + Decode,
{
	pub metadata: CellMetadata<BlockNumber>,
	// The KZG proof of the cell's data in blob.
	pub proof: KZGProof,
	// The data of the cell.
	pub data: [u8; 31],
}

impl<BlockNumber> Cell<BlockNumber>
where
	BlockNumber: Clone + sp_std::hash::Hash + Encode + Decode,
{
	pub fn new(metadata: CellMetadata<BlockNumber>, proof: KZGProof, data: &BlsScalar) -> Self {
		Self { metadata, proof, data: data.to_bytes_safe() }
	}

	pub fn index(&self) -> u32 {
		self.metadata.index()
	}

	pub fn piece_index(&self) -> u32 {
		self.metadata.piece_metadata.pos.to_u32()
	}
}

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
	Decode, Encode, PieceMetadata, PiecePosition, YPos, EXTENDED_SEGMENTS_PER_BLOB,
};
use melo_core_primitives::config::SEGMENTS_PER_BLOB;
use melo_das_primitives::{KZGCommitment, Segment, KZG};
use scale_info::TypeInfo;
use sp_core::RuntimeDebug;

#[derive(Default, Clone, Debug, PartialEq, Eq, Encode, Decode, TypeInfo)]
pub struct CellMetadata<BlockNumber>
where
	BlockNumber: Clone + sp_std::hash::Hash + Encode + Decode,
{
	pub piece_metadata: PieceMetadata<BlockNumber>,
	pub offset: u32,
}

impl<BlockNumber> CellMetadata<BlockNumber>
where
	BlockNumber: Clone + sp_std::hash::Hash + Encode + Decode,
{
	pub fn new(piece_metadata: PieceMetadata<BlockNumber>, offset: u32) -> Self {
		Self { piece_metadata, offset }
	}

	pub fn is_pair(&self, other: &Self) -> bool {
		let x_pos = YPos::from_u32(self.offset);
		let other_x_pos = YPos::from_u32(other.offset);
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
	pub seg: Segment,
}

impl<BlockNumber> Cell<BlockNumber>
where
	BlockNumber: Clone + sp_std::hash::Hash + Encode + Decode,
{
	pub fn new(metadata: CellMetadata<BlockNumber>, seg: Segment) -> Self {
		Self { metadata, seg }
	}

	pub fn piece_index(&self) -> u32 {
		self.metadata.piece_metadata.pos.to_u32()
	}

	pub fn verify_kzg_proof(&self, kzg: &KZG, commitment: &KZGCommitment) -> bool {
		self.seg.verify(kzg, commitment, SEGMENTS_PER_BLOB).unwrap_or(false)
	}
}

#[derive(Encode, Decode, RuntimeDebug, Default, Clone, PartialEq, Eq, TypeInfo)]
pub struct PreCell {
	pub seg: Segment,
	pub position: PiecePosition,
}

impl PreCell {
	pub fn new(position: PiecePosition, seg: Segment) -> Self {
		Self { position, seg }
	}

	pub fn piece_index(&self) -> u32 {
		self.position.to_u32()
	}

	pub fn verify_kzg_proof(&self, kzg: &KZG, commitment: &KZGCommitment) -> bool {
		self.seg.verify(kzg, commitment, EXTENDED_SEGMENTS_PER_BLOB).unwrap_or(false)
	}
}

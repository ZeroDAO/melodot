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
use crate::{CellMetadata, DasKv, YPos, ZValueManager};
use crate::{Decode, Encode, FarmerId, Segment, Vec, XValueManager};
#[cfg(feature = "std")]
use anyhow::{anyhow, Ok, Result};
use melo_das_primitives::Position;
use scale_info::TypeInfo;

// Import statements and module-level documentation are typically not included in inline
// documentation.

/// A structure representing a `Piece`, parameterized over a `BlockNumber`.
///
/// This struct encapsulates metadata and a list of segments that together define a `Piece`.
/// `BlockNumber` is a generic type that represents the block number in which the piece is involved.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct Piece<BlockNumber>
where
	BlockNumber: Clone + sp_std::hash::Hash,
{
	/// Metadata about the piece, including block number and position.
	pub metadata: PieceMetadata<BlockNumber>,

	/// A vector of segments that make up the piece.
	pub segments: Vec<Segment>,
}

/// An enumeration representing the position of a piece, either by row or column.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, TypeInfo)]
pub enum PiecePosition {
	Row(u32),
	Column(u32),
}

impl PiecePosition {
	/// Converts the `PiecePosition` enum to a `u32` value.
	/// Returns the row or column number based on the position type.
	pub fn to_u32(&self) -> u32 {
		match self {
			PiecePosition::Row(row) => *row,
			PiecePosition::Column(column) => *column,
		}
	}

	/// Creates a `PiecePosition` from a given `Position` as a row.
	pub fn from_row(position: &Position) -> Self {
		Self::Row(position.x)
	}

	/// Creates a `PiecePosition` from a given `Position` as a column.
	pub fn from_column(position: &Position) -> Self {
		Self::Column(position.y)
	}
}

impl Default for PiecePosition {
	/// Provides a default value for `PiecePosition`, defaulting to `Row(0)`.
	fn default() -> Self {
		PiecePosition::Row(0)
	}
}

/// Metadata for a piece, parameterized over a `BlockNumber`.
///
/// Includes information about the block number and the position of the piece.
#[derive(Debug, Default, Clone, PartialEq, Eq, Encode, Decode, TypeInfo)]
pub struct PieceMetadata<BlockNumber>
where
	BlockNumber: Clone + sp_std::hash::Hash,
{
	/// The block number associated with the piece.
	pub block_num: BlockNumber,

	/// The position of the piece, either in a row or a column.
	pub pos: PiecePosition,
}

impl<BlockNumber> PieceMetadata<BlockNumber>
where
	BlockNumber: Clone + sp_std::hash::Hash + Encode,
{
	/// Constructs new `PieceMetadata` with the specified block number and position.
	pub fn new(block_num: BlockNumber, pos: PiecePosition) -> Self {
		Self { block_num, pos }
	}

	/// Generates a key for the piece metadata, useful for storage or identification purposes.
	pub fn key(&self) -> Vec<u8> {
		Encode::encode(&self)
	}
}

impl<BlockNumber> Piece<BlockNumber>
where
	BlockNumber: Clone + sp_std::hash::Hash + Encode + Decode + PartialEq,
{
	/// Generates a key for the piece, primarily based on its metadata.
	pub fn key(&self) -> Vec<u8> {
		Encode::encode(&self.metadata)
	}

	/// Constructs a new `Piece` with the provided block number, position, and segments.
	pub fn new(blcok_num: BlockNumber, pos: PiecePosition, segments: &[Segment]) -> Self {
		let metadata = PieceMetadata { block_num: blcok_num, pos };
		Self { metadata, segments: segments.to_vec() }
	}

	/// Provides an iterator over the x-values of the segments, paired with references to the
	/// segments.
	///
	/// This is useful for processing or iterating over segments with their computed x-values.
	pub fn x_values_iterator<'a>(
		&'a self,
		farmer_id: &'a FarmerId,
	) -> impl Iterator<Item = (u32, &Segment)> + 'a {
		self.segments.iter().map(move |segment| {
			let y = XValueManager::<BlockNumber>::calculate_y(farmer_id, segment);
			(y, segment)
		})
	}

	/// Retrieves a segment at the specified position, if it exists.
	///
	/// Returns `None` if the position is out of bounds.
	pub fn cell(&self, pos: u32) -> Option<Segment> {
		if pos >= self.segments.len() as u32 {
			return None
		}
		Some(self.segments[pos as usize].clone())
	}

	/// Retrieves a segment at the specified position, if it exists.
	#[cfg(feature = "std")]
	pub fn get_cell(
		metadata: &CellMetadata<BlockNumber>,
		db: &mut impl DasKv,
	) -> Result<Option<Segment>> {
		db.get(&metadata.piece_metadata.key())
			.map(|data| {
				Decode::decode(&mut &data[..])
					.map_err(|e| anyhow!("Failed to decode Piece from database: {}", e))
					.map(|piece: Piece<BlockNumber>| piece.cell(metadata.offset))
			})
			.transpose()
			.map(|opt| opt.flatten())
	}

	/// Saves the `Piece` to the database. This process involves handling all data within the
	/// `Piece`, including calculating Y and Z values, and storing the corresponding index pairs.
	#[cfg(feature = "std")]
	pub fn save(&self, db: &mut impl DasKv, farmer_id: &FarmerId) -> Result<()> {
		let metadata_clone = self.metadata.clone();
		db.set(&self.key(), &self.encode());

		self.x_values_iterator(farmer_id).enumerate().try_for_each(
			|(index, (x, bls_scalar_ref))| {
				let cell_metadata =
					CellMetadata { piece_metadata: metadata_clone.clone(), offset: index as u32 };

				let x_value_manager =
					XValueManager::<BlockNumber>::new(&metadata_clone, index as u32, x);

				let x_pos = YPos::from_u32(index as u32);

				x_value_manager.save(db);

				let match_cells = x_value_manager.match_cells(db)?;

				for mc in match_cells {
					if let Some(seg) = Self::get_cell(&mc, db)? {
						match x_pos {
							YPos::Left(_) => {
								let z_value_manager =
									ZValueManager::new(&cell_metadata, &mc, bls_scalar_ref, &seg);
								z_value_manager.save(db);
							},
							YPos::Right(_) => {
								let z_value_manager =
									ZValueManager::new(&cell_metadata, &mc, &seg, bls_scalar_ref);
								z_value_manager.save(db);
							},
						}
					}
				}
				Ok(())
			},
		)?;
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use melo_das_db::mock_db::MockDb;

	#[test]
	fn test_piece_creation_and_key() {
		let block_num = 123;
		let position = PiecePosition::Row(1);
		let segment = Segment::default();
		let piece = Piece::new(block_num, position.clone(), &[segment]);

		assert_eq!(piece.metadata.block_num, block_num);
		assert_eq!(piece.metadata.pos, position);
		assert!(piece.segments.len() == 1);

		let key = piece.key();
		assert!(!key.is_empty());
	}

	#[test]
	fn test_piece_position() {
		let row_pos = PiecePosition::Row(10);
		assert_eq!(row_pos.to_u32(), 10);

		let col_pos = PiecePosition::Column(20);
		assert_eq!(col_pos.to_u32(), 20);

		let position = Position { x: 5, y: 15 };
		let row_position = PiecePosition::from_row(&position);
		assert_eq!(row_position, PiecePosition::Row(5));

		let col_position = PiecePosition::from_column(&position);
		assert_eq!(col_position, PiecePosition::Column(15));
	}

	#[test]
	fn test_piece_save() {
		let mut db = MockDb::new();
		let block_num = 123;
		let position = PiecePosition::Row(1);
		let segment = Segment::default();
		let piece = Piece::new(block_num, position, &[segment]);

		let farmer_id = FarmerId::default();

		assert!(piece.save(&mut db, &farmer_id).is_ok());

		let key = piece.key();
		assert!(db.contains(&key));

		if let Some(encoded_data) = db.get(&key) {
			let decoded_piece = Piece::<u32>::decode(&mut &encoded_data[..]).expect("Decode error");
			assert_eq!(decoded_piece, piece);
		} else {
			panic!("Piece not found in database");
		}
	}
}

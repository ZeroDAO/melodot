// Copyright 2023 ZeroDAO
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
extern crate alloc;

use alloc::{string::String, vec::Vec};
use derive_more::{AsMut, AsRef, From};
use rust_kzg_blst::utils::reverse_bit_order;

use crate::config::{FIELD_ELEMENTS_PER_BLOB, SEGMENT_LENGTH};
use crate::kzg::{
	BlsScalar, Cell, KZGProof, Polynomial, Position, KZG, KZGCommitment, ReprConvert,
};

#[derive(Debug, Default, Clone, PartialEq, Eq, From, AsRef, AsMut)]
pub struct Segment {
	pub position: Position,
	pub content: SegmentData,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, From, AsRef, AsMut)]
pub struct SegmentData {
	pub data: Vec<BlsScalar>,
	pub proof: KZGProof,
}

impl SegmentData {
	pub const SIZE: usize = SEGMENT_LENGTH;

	pub fn new(data: &[BlsScalar], proof: KZGProof) -> Self {
		// TODO: check data length
		let mut arr = [BlsScalar::default(); SEGMENT_LENGTH].to_vec();
		arr.copy_from_slice(&data[..SEGMENT_LENGTH]);
		Self { data: arr, proof }
	}

	pub fn size(&self) -> usize {
		Self::SIZE
	}

	pub fn from_data(
		positon: &Position,
		content: &[BlsScalar],
		kzg: &KZG,
		poly: &Polynomial,
		chunk_count: usize,
		n: usize,
	) -> Result<Self, String> {
		let i = kzg.get_kzg_index(chunk_count, positon.x as usize, n);
		kzg.compute_proof_multi(poly, i, FIELD_ELEMENTS_PER_BLOB)
			.map(|p| Self::new(content, p))
	}
}

impl Segment {
	pub const SIZE: usize = SEGMENT_LENGTH;

	pub fn new(position: Position, data: &[BlsScalar], proof: KZGProof) -> Self {
		let segment_data = SegmentData::new(data, proof);
		Self { position, content: segment_data }
	}
 
	pub fn verify(&self, kzg: &KZG, commitment: &KZGCommitment, count: usize) -> Result<bool, String> {
		let mut ys = BlsScalar::vec_to_repr(self.content.data.clone());
		reverse_bit_order(&mut ys);
		kzg.check_proof_multi(
			&commitment,
			self.position.x as usize,
			count,
			&ys,
			&self.content.proof,
			Self::SIZE
		)
	}

	pub fn get_cell_by_offset(&self, offset: usize) -> Cell {
		let x = self.position.x * (SEGMENT_LENGTH as u32) + (offset as u32);
		let position = Position { x, y: self.position.y };
		Cell { data: self.content.data[offset], position }
	}

	pub fn get_cell_by_index(&self, index: usize) -> Cell {
		let offset = index % SEGMENT_LENGTH;
		self.get_cell_by_offset(offset)
	}

	pub fn get_all_cells(&self) -> Vec<Cell> {
		let mut cells = Vec::with_capacity(SEGMENT_LENGTH);
		for i in 0..SEGMENT_LENGTH {
			cells.push(self.get_cell_by_offset(i));
		}
		cells
	}
}

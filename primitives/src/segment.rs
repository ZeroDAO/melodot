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

use crate::config::FIELD_ELEMENTS_PER_BLOB;
use crate::kzg::{
	BlsScalar, Cell, KZGCommitment, KZGProof, Polynomial, Position, ReprConvert, KZG,
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
	pub fn new(proof: KZGProof, size: usize) -> Self {
		// TODO: check data length
		let arr = vec![BlsScalar::default(); size];
		Self { data: arr, proof }
	}

	pub fn size(&self) -> usize {
		self.data.len()
	}

	pub fn from_data(
		positon: &Position,
		content: &[BlsScalar],
		kzg: &KZG,
		poly: &Polynomial,
		chunk_count: usize,
	) -> Result<Self, String> {
		// let i = kzg.get_kzg_index(chunk_count, positon.x as usize, content.len());
		kzg.compute_proof_multi(poly, positon.x as usize, chunk_count, content.len())
			.map(|p| Ok(Self { data: content.to_vec(), proof: p }))?
	}
}

impl Segment {
	pub fn new(position: Position, data: &[BlsScalar], proof: KZGProof) -> Self {
		let segment_data = SegmentData { data: data.to_vec(), proof };
		Self { position, content: segment_data }
	}

	pub fn size(&self) -> usize {
		self.content.data.len()
	}

	pub fn verify(
		&self,
		kzg: &KZG,
		commitment: &KZGCommitment,
		count: usize,
	) -> Result<bool, String> {
		let mut ys = BlsScalar::vec_to_repr(self.content.data.clone());
		reverse_bit_order(&mut ys);
		kzg.check_proof_multi(
			&commitment,
			self.position.x as usize,
			count,
			&ys,
			&self.content.proof,
			self.size(),
		)
	}

	// pub fn get_cell_by_offset(&self, offset: usize) -> Cell {
	// 	let x = self.position.x * (SEGMENT_LENGTH as u32) + (offset as u32);
	// 	let position = Position { x, y: self.position.y };
	// 	Cell { data: self.content.data[offset], position }
	// }

	// pub fn get_cell_by_index(&self, index: usize) -> Cell {
	// 	let offset = index % SEGMENT_LENGTH;
	// 	self.get_cell_by_offset(offset)
	// }

	// pub fn get_all_cells(&self) -> Vec<Cell> {
	// 	let mut cells = Vec::with_capacity(SEGMENT_LENGTH);
	// 	for i in 0..SEGMENT_LENGTH {
	// 		cells.push(self.get_cell_by_offset(i));
	// 	}
	// 	cells
	// }
}

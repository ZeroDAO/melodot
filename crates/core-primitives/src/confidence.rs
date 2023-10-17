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
use alloc::vec::Vec;
use codec::{Decode, Encode};
use rand::Rng;

use melo_das_db::traits::DasKv;
use melo_das_primitives::{config::FIELD_ELEMENTS_PER_BLOB, Position};

const CHUNK_COUNT: usize = 2 ^ 4;
const SAMPLES_PER_BLOB: usize = FIELD_ELEMENTS_PER_BLOB / CHUNK_COUNT;

/// Confidence trait defines methods related to the confidence of an application.
pub trait Confidence {
	/// Returns the confidence value.
	fn value(&self, base_factor: f64) -> f32;

	/// Constructs a unique ID.
	fn id(&self) -> Vec<u8>;

	/// Fetches `n` random sample positions from the `SAMPLES_PER_BLOB * total_rows` matrix.
	fn set_sample(&mut self, n: usize);

	/// Checks if the availability exceeds the provided threshold.
	fn exceeds_threshold(&self, base_factor: f64, threshold: f32) -> bool;

	/// Saves the current instance to the database.
	fn save(&self, db: &mut impl DasKv);

	/// Retrieves an instance from the database.
	fn get(id: &[u8], db: &mut impl DasKv) -> Option<Self>
	where
		Self: Sized;

	/// Removes the current instance from the database.
	fn remove(&self, db: &mut impl DasKv);
}

#[derive(Debug, Clone, Default, Decode, Encode)]
pub struct Sample {
	position: Position,
	is_availability: bool,
}

impl Sample {
	pub fn set_success(&mut self) {
		self.is_availability = true;
	}
}

pub const AVAILABILITY_THRESHOLD: f32 = 0.8;

#[derive(Debug, Clone, Decode, Encode, Default)]
pub struct ConfidenceBase {
	id: Vec<u8>,
	total_rows: u32,
	samples: Vec<Sample>,
}

impl Confidence for ConfidenceBase {
	fn value(&self, base_factor: f64) -> f32 {
		let success_count = self.samples.iter().filter(|&sample| sample.is_availability).count();
		calculate_confidence(success_count as u32, base_factor) as f32
	}

	fn id(&self) -> Vec<u8> {
		self.id.clone()
	}

	fn set_sample(&mut self, n: usize) {
		let mut rng = rand::thread_rng();
		let mut positions = Vec::with_capacity(n);

		while positions.len() < n {
			let x = rng.gen_range(0..SAMPLES_PER_BLOB) as u32;
			let y = rng.gen_range(0..self.total_rows);

			let pos = Position { x, y };

			if !positions.contains(&pos) {
				positions.push(pos);
			}
		}

		self.samples = positions
			.into_iter()
			.map(|pos| Sample { position: pos, is_availability: false })
			.collect();
	}

	fn exceeds_threshold(&self, base_factor: f64, threshold: f32) -> bool {
		self.value(base_factor) > threshold
	}

	fn save(&self, db: &mut impl DasKv) {
		db.set(&self.id(), &self.encode());
	}

	fn get(id: &[u8], db: &mut impl DasKv) -> Option<Self>
	where
		Self: Sized,
	{
		db.get(id).and_then(|encoded_data| Decode::decode(&mut &encoded_data[..]).ok())
	}

	fn remove(&self, db: &mut impl DasKv) {
		db.remove(&self.id());
	}
}

fn calculate_confidence(samples: u32, base_factor: f64) -> f64 {
	100f64 * (1f64 - base_factor.powi(samples as i32))
}

pub mod app_confidence {
	use super::*;

    pub const BASE_FACTOR: f64 = 0.5;

	pub fn id<BlockNum: Decode + Encode + Clone + Sized>(block_num: BlockNum, app_id: Vec<u8>) -> Vec<u8> {
        let mut id = app_id.clone();
		id.extend_from_slice(&block_num.encode());
        id
	}

	pub fn new_confidence<BlockNum: Decode + Encode + Clone + Sized>(
		block_num: BlockNum,
		app_id: Vec<u8>,
		total_rows: u32,
	) -> ConfidenceBase {
		let id = id(block_num.clone(), app_id.clone());
		ConfidenceBase { id, total_rows, samples: Vec::new() }
	}
}

pub mod block_confidence {
	use super::*;

    pub const BASE_FACTOR: f64 = 0.25;

	pub fn id(block_hash: Vec<u8>) -> Vec<u8> {
        block_hash
	}

	pub fn new_confidence(
        block_hash: Vec<u8>,
		total_rows: u32,
	) -> ConfidenceBase {
		let id = id(block_hash);
		ConfidenceBase { id, total_rows, samples: Vec::new() }
	}
}
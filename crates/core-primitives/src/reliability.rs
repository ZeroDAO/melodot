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

pub use sp_arithmetic::Permill;

use sp_arithmetic::traits::Saturating;

use crate::{KZGCommitment, String};
use alloc::vec::Vec;
use codec::{Decode, Encode};
#[cfg(feature = "std")]
use rand::Rng;

use melo_das_db::traits::DasKv;
use melo_das_primitives::{Position, Segment, KZG};

use crate::config::{BLOCK_AVAILABILITY_THRESHOLD, FIELD_ELEMENTS_PER_SEGMENT, SAMPLES_PER_BLOB};

/// 大于该数值的应用数据可用，应用数据抽样面临网络问题，允许一定概率的失败
/// TODO: 我们应该使用二项式分布？
pub const APP_AVAILABILITY_THRESHOLD_PERMILL: Permill = Permill::from_parts(900);
/// 最新处理的区块的键
const LATEST_PROCESSED_BLOCK_KEY: &[u8] = b"latestprocessedblock";

/// 应用的失败概率，这是一个千分比
pub const APP_FAILURE_PROBABILITY: Permill = Permill::from_parts(500);
/// 区块的失败概率，这是一个千分比
pub const BLOCK_FAILURE_PROBABILITY: Permill = Permill::from_parts(250);

#[cfg(feature = "std")]
pub trait ReliabilitySample {
	fn set_sample(&mut self, n: usize);
}

#[derive(Debug, Clone, Default, Decode, Encode)]
pub struct ReliabilityId(Vec<u8>);

impl ReliabilityId {
	pub fn block_confidence(block_hash: &[u8]) -> Self {
		Self(block_hash.into())
	}

	pub fn app_confidence(app_id: u32, nonce: u32) -> Self {
		let mut buffer = [0u8; 8];

		buffer[..4].copy_from_slice(&app_id.to_be_bytes());
		buffer[4..].copy_from_slice(&nonce.to_be_bytes());

		Self(buffer.into())
	}

	pub fn get_confidence(&self, db: &mut impl DasKv) -> Option<Reliability> {
		Reliability::get(self, db)
	}
}

pub struct ReliabilityManager<DB>
where
	DB: DasKv,
{
	db: DB,
}

impl<DB> ReliabilityManager<DB>
where
	DB: DasKv,
{
	pub fn new(db: DB) -> Self {
		Self { db }
	}

	pub fn get_last_processed_block(&mut self) -> Option<u32> {
		self.db.get(LATEST_PROCESSED_BLOCK_KEY).map(|data| {
			let mut buffer = [0u8; 4];
			buffer.copy_from_slice(&data);
			u32::from_be_bytes(buffer)
		})
	}

	pub fn set_last_processed_block(&mut self, block_num: u32) {
		self.db.set(LATEST_PROCESSED_BLOCK_KEY, &block_num.to_be_bytes());
	}
}

#[derive(Debug, Clone, Default, Decode, Encode)]
pub struct Sample {
	pub position: Position,
	pub is_availability: bool,
}

impl Sample {
	pub fn set_success(&mut self) {
		self.is_availability = true;
	}

	pub fn key(&self, app_id: u32, nonce: u32) -> Vec<u8> {
		sample_key(app_id, nonce, &self.position)
	}
}

#[derive(Debug, Clone, Copy, Decode, Encode)]
pub enum ReliabilityType {
	App,
	Block,
}

impl Default for ReliabilityType {
	fn default() -> Self {
		ReliabilityType::App
	}
}

impl ReliabilityType {
	pub fn failure_probability(&self) -> Permill {
		match self {
			ReliabilityType::App => APP_FAILURE_PROBABILITY,
			ReliabilityType::Block => BLOCK_FAILURE_PROBABILITY,
		}
	}

	pub fn is_availability(&self, total_count: u32, success_count: u32) -> bool {
		match self {
			ReliabilityType::App =>
				success_count > APP_AVAILABILITY_THRESHOLD_PERMILL.mul_floor(total_count),
			ReliabilityType::Block => success_count >= BLOCK_AVAILABILITY_THRESHOLD,
		}
	}
}

#[derive(Debug, Clone, Decode, Encode, Default)]
pub struct Reliability {
	pub samples: Vec<Sample>,
	pub commitments: Vec<KZGCommitment>,
	pub confidence_type: ReliabilityType,
}

impl Reliability {
	pub fn new(confidence_type: ReliabilityType, commitments: &[KZGCommitment]) -> Self {
		Reliability { samples: Vec::new(), commitments: commitments.to_vec(), confidence_type }
	}

	pub fn success_count(&self) -> usize {
		self.samples.iter().filter(|&sample| sample.is_availability).count()
	}

	pub fn value(&self) -> Option<u32> {
		match self.confidence_type {
			ReliabilityType::App => None,
			ReliabilityType::Block => match self.samples.len() {
				0 => None,
				_ => {
					let failure_probability = self.confidence_type.failure_probability();
					let success_count =
						self.samples.iter().filter(|&sample| sample.is_availability).count();
					Some(calculate_confidence(success_count as u32, failure_probability))
				},
			},
		}
	}

	pub fn is_availability(&self) -> bool {
		self.confidence_type
			.is_availability(self.samples.len() as u32, self.success_count() as u32)
	}

	pub fn save(&self, id: &ReliabilityId, db: &mut impl DasKv) {
		db.set(&id.0, &self.encode());
	}

	pub fn get(id: &ReliabilityId, db: &mut impl DasKv) -> Option<Self>
	where
		Self: Sized,
	{
		db.get(&id.0)
			.and_then(|encoded_data| Decode::decode(&mut &encoded_data[..]).ok())
	}

	pub fn remove(&self, id: &ReliabilityId, db: &mut impl DasKv) {
		db.remove(&id.0);
	}

	pub fn set_sample_success(&mut self, position: Position) {
		if let Some(sample) = self.samples.iter_mut().find(|sample| sample.position == position) {
			sample.set_success();
		}
	}

	pub fn verify_sample(&self, position: Position, segment: &Segment) -> Result<bool, String> {
		let kzg = KZG::default_embedded();
		if position.y >= self.commitments.len() as u32 {
			return Ok(false)
		}
		let commitment = self.commitments[position.y as usize];
		segment.checked()?.verify(&kzg, &commitment, FIELD_ELEMENTS_PER_SEGMENT)
	}
}

#[cfg(feature = "std")]
impl ReliabilitySample for Reliability {
	fn set_sample(&mut self, n: usize) {
		let mut rng = rand::thread_rng();
		let mut positions = Vec::with_capacity(n);

		if self.commitments.is_empty() {
			return
		}

		while positions.len() < n {
			let x = rng.gen_range(0..SAMPLES_PER_BLOB) as u32;
			let y = rng.gen_range(0..self.commitments.len() as u32);

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
}

fn calculate_confidence(samples: u32, failure_probability: Permill) -> u32 {
	let one = Permill::one();
	let base_power_sample = failure_probability.saturating_pow(samples as usize);
	one.saturating_sub(base_power_sample).deconstruct()
}

pub fn sample_key(app_id: u32, nonce: u32, position: &Position) -> Vec<u8> {
	let mut key = Vec::new();
	key.extend_from_slice(&app_id.to_be_bytes());
	key.extend_from_slice(&nonce.to_be_bytes());
	key.extend_from_slice(&position.encode());
	key
}

pub fn sample_key_from_block(block_hash: &[u8], position: &Position) -> Vec<u8> {
	let mut key = Vec::new();
	key.extend_from_slice(block_hash);
	key.extend_from_slice(&position.encode());
	key
}

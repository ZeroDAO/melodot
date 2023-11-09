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

use crate::{AppLookup, KZGCommitment, String};
use alloc::vec::Vec;
use codec::{Decode, Encode};
#[cfg(feature = "std")]
use rand::Rng;

use melo_das_db::traits::DasKv;
use melo_das_primitives::{Position, Segment, KZG};

use crate::config::{
	BLOCK_AVAILABILITY_THRESHOLD, EXTENDED_SEGMENTS_PER_BLOB, FIELD_ELEMENTS_PER_SEGMENT,
};

/// 大于该数值的应用数据可用，应用数据抽样面临网络问题，允许一定概率的失败
/// TODO: 我们应该使用二项式分布？
pub const APP_AVAILABILITY_THRESHOLD_PERMILL: Permill = Permill::from_parts(900_000);
/// 最新处理的区块的键
const LATEST_PROCESSED_BLOCK_KEY: &[u8] = b"latestprocessedblock";

/// 应用的失败概率，这是一个千分比
pub const APP_FAILURE_PROBABILITY: Permill = Permill::from_parts(500_000);
/// 区块的失败概率，这是一个千分比
pub const BLOCK_FAILURE_PROBABILITY: Permill = Permill::from_parts(250_000);

#[cfg(feature = "std")]
pub trait ReliabilitySample {
	fn set_sample(
		&mut self,
		n: usize,
		app_lookups: &[AppLookup],
		block_hash: Option<&[u8]>,
	) -> Result<Vec<KZGCommitment>, String>;
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
pub struct SampleId(Vec<u8>);

impl SampleId {
	pub fn block_sample(block_hash: &[u8], position: &Position) -> Self {
		Self(sample_key_from_block(block_hash, position))
	}

	pub fn app_sample(app_id: u32, nonce: u32, position: &Position) -> Self {
		Self(sample_key(app_id, nonce, position))
	}
}

#[derive(Debug, Clone, Default, Decode, Encode)]
pub struct Sample {
	pub id: SampleId,
	pub position: Position,
	pub is_availability: bool,
}

impl Sample {
	pub fn get_id(&self) -> &[u8] {
		&self.id.0
	}

	pub fn set_success(&mut self) {
		self.is_availability = true;
	}

	pub fn key(&self, app_id: u32, nonce: u32) -> Vec<u8> {
		sample_key(app_id, nonce, &self.position)
	}
}

#[derive(Debug, Clone, Copy, Decode, Encode, Default)]
pub enum ReliabilityType {
	#[default]
	App,
	Block,
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

	/// Calculates the maximum number of consecutive successful samples.
	///
	/// This method iterates through the `samples` vector and counts the length of the longest
	/// sequence of consecutive samples where `is_availability` is `true`.
	///
	/// # Returns
	///
	/// Returns the count of the longest consecutive successful samples as a `usize`.
	pub fn success_count(&self) -> usize {
		self.samples
			.iter()
			.fold((0, 0), |(max_count, curr_count), sample| {
				if sample.is_availability {
					(max_count.max(curr_count + 1), curr_count + 1)
				} else {
					(max_count, 0)
				}
			})
			.0
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
	fn set_sample(
		&mut self,
		n: usize,
		app_lookups: &[AppLookup],
		block_hash: Option<&[u8]>,
	) -> Result<Vec<KZGCommitment>, String> {
		let mut rng = rand::thread_rng();
		let mut positions = Vec::with_capacity(n);

		let column_count = self.commitments.len() as u32;

		if column_count == 0 {
			return Ok(vec![])
		}

		let mut commitments = Vec::with_capacity(n);

		while positions.len() < n {
			let x = rng.gen_range(0..EXTENDED_SEGMENTS_PER_BLOB) as u32;
			let y = rng.gen_range(0..column_count);
		
			let pos = Position { x, y };
		
			if !positions.contains(&pos) {
				commitments.push(self.commitments[pos.y as usize]);
				positions.push(pos);
			}
		}

		self.samples = match self.confidence_type {
			ReliabilityType::App => app_lookups
				.first()
				.ok_or_else(|| "No app lookups available".to_string())
				.and_then(|app_lookup| {
					positions
						.into_iter()
						.map(|pos| {
							let key = sample_key(app_lookup.app_id, app_lookup.nonce, &pos);
							Ok(Sample { id: SampleId(key), position: pos, is_availability: false })
						})
						.collect::<Result<Vec<_>, String>>()
				}),
			ReliabilityType::Block => {
				let block_hash = block_hash.ok_or_else(|| "Block hash not provided".to_string())?;
				positions
					.into_iter()
					.map(|pos| {
						if pos.y < column_count / 2 {
							AppLookup::get_lookup(app_lookups, pos.y)
								.ok_or_else(|| "AppLookup not found for position".to_string())
								.map(|(lookup, relative_y)| {
									let relative_pos = Position { x: pos.x, y: relative_y };
									let key =
										sample_key(lookup.app_id, lookup.nonce, &relative_pos);
									Sample {
										id: SampleId(key),
										position: pos,
										is_availability: false,
									}
								})
						} else {
							let key = sample_key_from_block(block_hash, &pos);
							Ok(Sample { id: SampleId(key), position: pos, is_availability: false })
						}
					})
					.collect::<Result<Vec<_>, String>>()
			},
		}?;

		Ok(commitments)
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

#[cfg(test)]
mod tests {
	use super::*;
	use melo_das_db::traits::DasKv;

	struct MockDb {
		storage: std::collections::HashMap<Vec<u8>, Vec<u8>>,
	}

	impl MockDb {
		fn new() -> Self {
			MockDb { storage: std::collections::HashMap::new() }
		}
	}

	impl DasKv for MockDb {
		fn get(&mut self, key: &[u8]) -> Option<Vec<u8>> {
			self.storage.get(key).cloned()
		}

		fn set(&mut self, key: &[u8], value: &[u8]) {
			self.storage.insert(key.to_vec(), value.to_vec());
		}

		fn remove(&mut self, key: &[u8]) {
			self.storage.remove(key);
		}

		fn contains(&mut self, key: &[u8]) -> bool {
			self.storage.contains_key(key)
		}

		fn compare_and_set(
			&mut self,
			key: &[u8],
			old_value: Option<&[u8]>,
			new_value: &[u8],
		) -> bool {
			match (self.get(key), old_value) {
				(Some(current_value), Some(old_value)) =>
					if current_value == old_value {
						self.set(key, new_value);
						true
					} else {
						false
					},
				(None, None) => {
					self.set(key, new_value);
					true
				},
				_ => false,
			}
		}
	}

	#[test]
	fn test_reliability_id_get_confidence() {
		let mut db = MockDb::new();
		let id = ReliabilityId::block_confidence(&[0, 1, 2, 3]);

		// This should return None as the reliability has not been set yet
		assert!(id.get_confidence(&mut db).is_none());

		// Now let's set a reliability
		let reliability = Reliability {
			samples: vec![],
			commitments: vec![],
			confidence_type: ReliabilityType::Block,
		};
		reliability.save(&id, &mut db);

		// Should be able to retrieve the reliability
		assert!(id.get_confidence(&mut db).is_some());
	}

	#[test]
	fn test_block_confidence() {
		let block_hash = [1, 2, 3, 4];
		let reliability_id = ReliabilityId::block_confidence(&block_hash);

		assert_eq!(reliability_id.0, block_hash.to_vec());
	}

	#[test]
	fn test_app_confidence() {
		let app_id = 1234;
		let nonce = 5678;
		let reliability_id = ReliabilityId::app_confidence(app_id, nonce);

		assert_eq!(reliability_id.0[..4], app_id.to_be_bytes());
		assert_eq!(reliability_id.0[4..], nonce.to_be_bytes());
	}

	#[test]
	fn test_set_and_get_last_processed_block() {
		let db = MockDb::new();
		let mut manager = ReliabilityManager::new(db);

		let block_num = 12345u32;
		manager.set_last_processed_block(block_num);

		assert_eq!(manager.get_last_processed_block(), Some(block_num));
	}

	#[test]
	fn test_reliability_success_count() {
		let mut reliability = Reliability::new(ReliabilityType::App, &[]);
		reliability.samples.push(Sample {
			id: SampleId(vec![1]),
			position: Position { x: 0, y: 0 },
			is_availability: true,
		});

		assert_eq!(reliability.success_count(), 1);
	}

	#[test]
	fn test_set_sample_with_empty_commitments() {
		let mut reliability = Reliability::default();
		reliability.confidence_type = ReliabilityType::Block;

		// Assuming ReliabilitySample is implemented for Reliability
		let result = reliability.set_sample(10, &[], None);

		assert!(result.is_ok());
		let commitments = result.unwrap();
		assert_eq!(commitments.len(), 0);
	}

	#[test]
	fn test_set_sample_app() {
		let mut reliability = Reliability::default();
		reliability.confidence_type = ReliabilityType::App;
		reliability.commitments = vec![
			KZGCommitment::default(),
			KZGCommitment::default(),
			KZGCommitment::default(),
			KZGCommitment::default(),
			KZGCommitment::default(),
		];

		let app_lookups = vec![AppLookup { app_id: 1, nonce: 3, count: 5 }];

		// Assuming ReliabilitySample is implemented for Reliability
		let result = reliability.set_sample(10, &app_lookups, None);

		assert!(result.is_ok());

		let commitments = result.unwrap();

		assert_eq!(commitments.len(), 10);

		let mut positions = Vec::new();
		for sample in reliability.samples.iter() {
			assert_eq!(sample.is_availability, false);
			assert!(!positions.contains(&sample.position));

			let key = sample_key(1, 3, &sample.position);
			assert_eq!(sample.id.0, key);

			positions.push(sample.position.clone());
		}

		assert_eq!(positions.len(), 10);
	}

	#[test]
	fn test_set_sample_block() {
		let mut reliability = Reliability::default();
		reliability.confidence_type = ReliabilityType::Block;
		reliability.commitments = vec![
			KZGCommitment::default(),
			KZGCommitment::default(),
			KZGCommitment::default(),
			KZGCommitment::default(),
			KZGCommitment::default(),
			KZGCommitment::default(),
			KZGCommitment::default(),
			KZGCommitment::default(),
		];

		let app_lookups = vec![
			AppLookup { app_id: 1, nonce: 3, count: 1 },
			AppLookup { app_id: 2, nonce: 5, count: 2 },
			AppLookup { app_id: 3, nonce: 1, count: 1 },
		];

		let block_hash = vec![0u8; 32];

		let n = 10;

		let result = reliability.set_sample(n, &app_lookups, Some(&block_hash));

		assert!(result.is_ok());

		let commitments = result.unwrap();
		assert_eq!(commitments.len(), n);

		let mut positions = Vec::new();
		for sample in reliability.samples.iter() {
			assert_eq!(sample.is_availability, false);

			assert!(!positions.contains(&sample.position));

			if sample.position.y >= 4 {
				let key = sample_key_from_block(&block_hash, &sample.position);
				assert_eq!(sample.id.0, key);
			}

			positions.push(sample.position.clone());
		}

		assert_eq!(positions.len(), n);
	}

	#[test]
	fn test_max_consecutive_success_count() {
		let mut samples = Vec::new();
		samples.push(Sample {
			id: SampleId(vec![]),
			position: Position::default(),
			is_availability: true,
		});
		samples.push(Sample {
			id: SampleId(vec![]),
			position: Position::default(),
			is_availability: true,
		});
		samples.push(Sample {
			id: SampleId(vec![]),
			position: Position::default(),
			is_availability: false,
		});
		samples.push(Sample {
			id: SampleId(vec![]),
			position: Position::default(),
			is_availability: true,
		});
		samples.push(Sample {
			id: SampleId(vec![]),
			position: Position::default(),
			is_availability: true,
		});
		let reliability = Reliability {
			samples,
			commitments: Vec::new(),
			confidence_type: ReliabilityType::default(),
		};

		assert_eq!(reliability.success_count(), 2);
	}
}

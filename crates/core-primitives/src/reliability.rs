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

#[cfg(feature = "std")]
use crate::config::EXTENDED_SEGMENTS_PER_BLOB;
#[cfg(feature = "std")]
use crate::AppLookup;
use crate::{KZGCommitment, String};
use alloc::vec::Vec;
use codec::{Decode, Encode};
use melo_das_db::traits::DasKv;
use melo_das_primitives::{Position, Segment, KZG};
#[cfg(feature = "std")]
use rand::Rng;

use crate::config::{BLOCK_AVAILABILITY_THRESHOLD, FIELD_ELEMENTS_PER_SEGMENT};

/// Application data is available if it is greater than this value. The application data sampling
/// faces network issues, allowing a certain probability of failure. TODO: Should we use a binomial
/// distribution?
pub const APP_AVAILABILITY_THRESHOLD_PERMILL: Permill = Permill::from_parts(900_000);
/// The key of the latest processed block
pub const LATEST_PROCESSED_BLOCK_KEY: &[u8] = b"latestprocessedblock";
/// The failure probability of the application, this is a permillage
pub const APP_FAILURE_PROBABILITY: Permill = Permill::from_parts(500_000);
/// The failure probability of the block, this is a permillage
pub const BLOCK_FAILURE_PROBABILITY: Permill = Permill::from_parts(250_000);

/// A trait for setting reliability samples.
#[cfg(feature = "std")]
pub trait ReliabilitySample {
	fn set_sample(
		&mut self,
		n: usize,
		app_lookups: &[AppLookup],
		block_hash: Option<&[u8]>,
	) -> Result<Vec<KZGCommitment>, String>;
}

/// Creates a new ReliabilityId based on the block hash.
#[derive(Debug, Clone, Default, Decode, Encode)]
pub struct ReliabilityId(pub Vec<u8>);

/// Implementation of ReliabilityId
impl ReliabilityId {
	/// Returns a new ReliabilityId with block confidence
	pub fn block_confidence(block_hash: &[u8]) -> Self {
		Self(block_hash.into())
	}

	/// Returns a new ReliabilityId with app confidence
	pub fn app_confidence(app_id: u32, nonce: u32) -> Self {
		let mut buffer = [0u8; 8];

		buffer[..4].copy_from_slice(&app_id.to_be_bytes());
		buffer[4..].copy_from_slice(&nonce.to_be_bytes());

		Self(buffer.into())
	}

	/// Returns the reliability of the current ReliabilityId from the database
	pub fn get_confidence(&self, db: &mut impl DasKv) -> Option<Reliability> {
		Reliability::get(self, db)
	}

	pub fn get_last(db: &mut impl DasKv) -> Option<LastProcessedBlock<u32>> {
		db.get(LATEST_PROCESSED_BLOCK_KEY).map(|data| {
			let last_processed_block = LastProcessedBlock::decode(&mut &data[..]).unwrap();
			last_processed_block
		})
	}

	pub fn set_last_processed_block<Number>(
		db: &mut impl DasKv,
		block_num: Number,
		block_hash: &[u8],
	) where
		Number: Encode + Decode + PartialOrd,
	{
		let last_processed_block = LastProcessedBlock { block_num, block_hash: block_hash.into() };
		db.set(LATEST_PROCESSED_BLOCK_KEY, last_processed_block.encode().as_slice());
	}
}

#[derive(Debug, Clone, Default, Decode, Encode)]
pub struct LastProcessedBlock<Number>
where
	Number: Encode + Decode + PartialOrd,
{
	pub block_num: Number,
	pub block_hash: Vec<u8>,
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
			let last_processed_block = LastProcessedBlock::decode(&mut &data[..]).unwrap();
			last_processed_block.block_num
		})
	}
}

#[derive(Debug, Clone, Default, Decode, Encode)]
pub struct SampleId(Vec<u8>);

impl SampleId {
	/// Creates a new `SampleId` for a block sample.
	///
	/// # Arguments
	///
	/// * `block_hash` - The hash of the block.
	/// * `position` - The position of the sample in the block.
	pub fn block_sample(block_hash: &[u8], position: &Position) -> Self {
		Self(sample_key_from_block(block_hash, position))
	}

	/// Creates a new `SampleId` for an app sample.
	///
	/// # Arguments
	///
	/// * `app_id` - The ID of the app.
	/// * `nonce` - The nonce of the app.
	/// * `position` - The position of the sample in the app.
	pub fn app_sample(app_id: u32, nonce: u32, position: &Position) -> Self {
		Self(sample_key(app_id, nonce, position))
	}
}

/// A struct representing a sample with an ID, position, and availability status.
#[derive(Debug, Clone, Default, Decode, Encode)]
pub struct Sample {
	/// The ID of the sample.
	pub id: SampleId,
	/// The position of the sample. When the sample is an app sample, the position is relative to
	/// the app. When the sample is a block sample, the position is relative to the block.
	pub position: Position,
	/// The availability status of the sample.
	pub is_availability: bool,
}

impl Sample {
	/// Returns the ID of the sample.
	pub fn get_id(&self) -> &[u8] {
		&self.id.0
	}

	/// Sets the availability status of the sample to true.
	pub fn set_success(&mut self) {
		self.is_availability = true;
	}

	/// Returns the key of the sample given an app ID and nonce.
	pub fn key(&self, app_id: u32, nonce: u32) -> Vec<u8> {
		sample_key(app_id, nonce, &self.position)
	}
}

/// An enum representing the type of reliability, either app or block.
#[derive(Debug, Clone, Copy, Decode, Encode, Default)]
pub enum ReliabilityType {
	#[default]
	App,
	Block,
}

/// Implementation of ReliabilityType
impl ReliabilityType {
	/// Returns the failure probability of the reliability type.
	pub fn failure_probability(&self) -> Permill {
		match self {
			ReliabilityType::App => APP_FAILURE_PROBABILITY,
			ReliabilityType::Block => BLOCK_FAILURE_PROBABILITY,
		}
	}

	/// Returns whether the reliability type is available given the total count and success count.
	pub fn is_availability(&self, total_count: u32, success_count: u32) -> bool {
		match self {
			ReliabilityType::App =>
				success_count > APP_AVAILABILITY_THRESHOLD_PERMILL.mul_floor(total_count),
			ReliabilityType::Block => success_count >= BLOCK_AVAILABILITY_THRESHOLD,
		}
	}
}

/// This module contains the implementation of reliability related structs and enums.
///
/// `Reliability` is a struct that contains a vector of `Sample`s, a vector of `KZGCommitment`s, and
/// a `ReliabilityType`. It provides methods to calculate the maximum number of consecutive
/// successful samples, the value of the reliability, and whether the reliability is available or
/// not.
#[derive(Debug, Clone, Decode, Encode, Default)]
pub struct Reliability {
	/// `Sample` represents a single reliability sample, which contains an ID, a position, and a
	/// boolean indicating whether the sample is available or not.
	pub samples: Vec<Sample>,
	/// `KZGCommitment` is a struct that contains a commitment and a proof.
	pub commitments: Vec<KZGCommitment>,
	/// `ReliabilityType` is an enum that represents the type of reliability, either App or Block.
	pub confidence_type: ReliabilityType,
}

impl Reliability {
	/// Creates a new instance of `Reliability`.
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

	/// Calculates the value of the reliability. The value is calculated using the formula:
	/// `1 - failure_probability ^ success_count`.
	/// If the reliability type is App, then the value is always `None`.
	/// If the reliability type is Block, then the value is calculated using the formula above.
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

	/// Returns whether the reliability is available or not.
	pub fn is_availability(&self) -> bool {
		self.confidence_type
			.is_availability(self.samples.len() as u32, self.success_count() as u32)
	}

	/// Saves the reliability to the database.
	pub fn save(&self, id: &ReliabilityId, db: &mut impl DasKv) {
		db.set(&id.0, &self.encode());
	}

	/// Returns the reliability from the database. If the reliability is not found, then `None` is
	/// returned.
	pub fn get(id: &ReliabilityId, db: &mut impl DasKv) -> Option<Self>
	where
		Self: Sized,
	{
		db.get(&id.0)
			.and_then(|encoded_data| Decode::decode(&mut &encoded_data[..]).ok())
	}

	/// Removes the reliability from the database.
	pub fn remove(&self, id: &ReliabilityId, db: &mut impl DasKv) {
		db.remove(&id.0);
	}

	/// Sets the availability status of the sample with the given position to true.
	pub fn set_sample_success(&mut self, position: Position) {
		if let Some(sample) = self.samples.iter_mut().find(|sample| sample.position == position) {
			sample.set_success();
		}
	}

	/// Verifies the sample with the given position and segment. Returns `Ok(true)` if the sample
	/// is verified, otherwise `Ok(false)`. If the sample is not found, then `Err` is returned.
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

/// Returns the key of the sample given an app ID, nonce, and position.
pub fn sample_key(app_id: u32, nonce: u32, position: &Position) -> Vec<u8> {
	let mut key = Vec::new();
	key.extend_from_slice(&app_id.to_be_bytes());
	key.extend_from_slice(&nonce.to_be_bytes());
	key.extend_from_slice(&position.encode());
	key
}

/// Returns the key of the sample given a block hash and position.
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

	// #[test]
	// fn test_set_and_get_last_processed_block() {
	// 	let db = MockDb::new();
	// 	let mut manager = ReliabilityManager::new(db);

	// 	let block_num = 12345u32;
	// 	manager.set_last_processed_block(block_num);

	// 	assert_eq!(manager.get_last_processed_block(), Some(block_num));
	// }

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

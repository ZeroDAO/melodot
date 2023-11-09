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

use codec::Encode;
use itertools::Itertools;
use melo_erasure_coding::bytes_to_segments;

use crate::{
	anyhow, sample_key, sample_key_from_block, Arc, Context, KZGCommitment, Ok, Position, Result,
	Sample, Segment, SegmentData, EXTENDED_SEGMENTS_PER_BLOB, FIELD_ELEMENTS_PER_BLOB,
	SEGMENTS_PER_BLOB,
};
use melo_core_primitives::{
	config::FIELD_ELEMENTS_PER_SEGMENT, traits::HeaderWithCommitment, Decode,
};
use melo_das_network::{KademliaKey, Service as DasNetworkService};
use melo_das_primitives::KZG;
use melo_erasure_coding::{
	extend_col::extend_segments_col as extend,
	recovery::recovery_order_row_from_segments as recovery,
};
use sp_api::HeaderT;

#[async_trait::async_trait]
pub trait DasNetworkOperations {
	async fn put_ext_segments<Header>(&self, segments: &[Segment], header: &Header) -> Result<()>
	where
		Header: HeaderT;

	async fn put_app_segments(&self, segments: &[Segment], app_id: u32, nonce: u32) -> Result<()>;

	async fn put_bytes(&self, bytes: &[u8], app_id: u32, nonce: u32) -> Result<()>;

	async fn fetch_segment_data(
		&self,
		app_id: u32,
		nonce: u32,
		position: &Position,
		commitment: &KZGCommitment,
	) -> Option<SegmentData>;

	async fn fetch_sample(
		&self,
		sample: &Sample,
		commitment: &KZGCommitment,
	) -> Option<SegmentData>;

	async fn fetch_block<Header>(
		&self,
		header: &Header,
	) -> Result<(Vec<Option<Segment>>, Vec<usize>, bool)>
	where
		Header: HeaderWithCommitment + HeaderT;

	fn extend_segments_col(&self, segments: &[Segment]) -> Result<Vec<Segment>>;

	fn recovery_order_row_from_segments(
		&self,
		segments: &[Option<Segment>],
	) -> Result<Vec<Segment>>;

	fn kzg(&self) -> Arc<KZG>;

	async fn remove_records(&self, keys: Vec<&[u8]>) -> Result<()>;
}

#[derive(Clone, Debug)]
pub struct DasNetworkServiceWrapper {
	network: Arc<DasNetworkService>,
	pub kzg: Arc<KZG>,
}

impl DasNetworkServiceWrapper {
	pub fn new(network: Arc<DasNetworkService>, kzg: Arc<KZG>) -> Self {
		DasNetworkServiceWrapper { network, kzg }
	}

	async fn fetch_value(
		&self,
		key: &[u8],
		position: &Position,
		commitment: &KZGCommitment,
	) -> Option<SegmentData> {
		let values = self.network.get_value(KademliaKey::new(&key)).await.ok()?;
		self.verify_values(&values, commitment, position).map(|segment| segment.content)
	}

	pub fn prepare_keys<Header>(&self, header: &Header) -> Result<Vec<KademliaKey>>
	where
		Header: HeaderWithCommitment + HeaderT,
	{
		let keys = header
			.extension()
			.app_lookup
			.iter()
			.flat_map(|app_lookup| {
				(0..EXTENDED_SEGMENTS_PER_BLOB).flat_map(move |x| {
					(0..app_lookup.count).map(move |y| {
						let position = Position { x: x as u32, y: y as u32 };
						let key = sample_key(app_lookup.app_id, app_lookup.nonce, &position);
						KademliaKey::new(&key)
					})
				})
			})
			.collect::<Vec<_>>();
		Ok(keys)
	}

	pub fn verify_values(
		&self,
		values: &[Vec<u8>],
		commitment: &KZGCommitment,
		position: &Position,
	) -> Option<Segment> {
		verify_values(&self.kzg, values, commitment, position)
	}
}

#[async_trait::async_trait]
impl DasNetworkOperations for DasNetworkServiceWrapper {
	fn kzg(&self) -> Arc<KZG> {
		self.kzg.clone()
	}

	fn extend_segments_col(&self, segments: &[Segment]) -> Result<Vec<Segment>> {
		extend(self.kzg.get_fs(), &segments.to_vec()).map_err(|e| anyhow!(e))
	}

	fn recovery_order_row_from_segments(
		&self,
		segments: &[Option<Segment>],
	) -> Result<Vec<Segment>> {
		recovery(segments, &self.kzg).map_err(|e| anyhow!(e))
	}

	async fn put_ext_segments<Header>(&self, segments: &[Segment], header: &Header) -> Result<()>
	where
		Header: HeaderT,
	{
		let values = segments
			.iter()
			.map(|segment| {
				let key = KademliaKey::new(&sample_key_from_block(
					&header.hash().encode(),
					&segment.position,
				));
				let value = segment.content.encode();
				(key, value)
			})
			.collect::<Vec<_>>();
		self.network.put_values(values).await?;
		Ok(())
	}

	async fn put_app_segments(&self, segments: &[Segment], app_id: u32, nonce: u32) -> Result<()> {
		let values = segments
			.iter()
			.map(|segment| {
				let key = KademliaKey::new(&sample_key(app_id, nonce, &segment.position));
				let value = segment.content.encode();
				(key, value)
			})
			.collect::<Vec<_>>();
		self.network.put_values(values).await?;
		Ok(())
	}

	async fn put_bytes(&self, bytes: &[u8], app_id: u32, nonce: u32) -> Result<()> {
		let segments = bytes_to_segments(
			bytes,
			FIELD_ELEMENTS_PER_BLOB,
			FIELD_ELEMENTS_PER_SEGMENT,
			&self.kzg,
		)
		.map_err(|e| anyhow!(e))?;
		self.put_app_segments(&segments, app_id, nonce).await
	}

	async fn fetch_segment_data(
		&self,
		app_id: u32,
		nonce: u32,
		position: &Position,
		commitment: &KZGCommitment,
	) -> Option<SegmentData> {
		let key = sample_key(app_id, nonce, position);
		self.fetch_value(&key, position, commitment).await
	}

	async fn fetch_sample(
		&self,
		sample: &Sample,
		commitment: &KZGCommitment,
	) -> Option<SegmentData> {
		self.fetch_value(sample.get_id(), &sample.position, commitment).await
	}

	async fn fetch_block<Header>(
		&self,
		header: &Header,
	) -> Result<(Vec<Option<Segment>>, Vec<usize>, bool)>
	where
		Header: HeaderWithCommitment + HeaderT,
	{
		let commitments = header.commitments().context("Header does not contain commitments.")?;
		let keys = self.prepare_keys(header)?;

		let values_set = self.network.get_values(&keys).await?;

		values_set_handler(&values_set, &commitments, &self.kzg)
	}

	async fn remove_records(&self, keys: Vec<&[u8]>) -> Result<()> {
		let keys = keys.into_iter().map(|key| KademliaKey::new(&key)).collect::<Vec<_>>();
		self.network.remove_records(&keys).await
	}
}

fn values_set_handler(
	values_set: &[Option<Vec<Vec<u8>>>],
	commitments: &[KZGCommitment],
	kzg: &KZG,
) -> Result<(Vec<Option<Segment>>, Vec<usize>, bool)> {

	if values_set.is_empty() || commitments.is_empty() {
		return Ok((vec![], vec![], false))
	}

	let mut need_reconstruct = vec![];
	let mut is_availability = true;

	let all_segments: Vec<Option<Segment>> = values_set
		.iter()
		.chunks(EXTENDED_SEGMENTS_PER_BLOB)
		.into_iter()
		.enumerate()
		.flat_map(|(y, chunk)| {
			if !is_availability {
				return vec![None; EXTENDED_SEGMENTS_PER_BLOB]
			}

			let segments = chunk
				.enumerate()
				.map(|(x, values)| match values {
					Some(values) => verify_values(
						kzg,
						values,
						&commitments[y],
						&Position { x: x as u32, y: y as u32 },
					),
					None => None,
				})
				.collect::<Vec<_>>();

			let available_segments = segments.iter().filter(|s| s.is_some()).count();

			if available_segments >= SEGMENTS_PER_BLOB {
				if available_segments < EXTENDED_SEGMENTS_PER_BLOB {
					need_reconstruct.push(y);
				}
			} else {
				is_availability = false;
			}
			segments
		})
		.collect();

	Ok((all_segments, need_reconstruct, is_availability))
}

fn verify_values(
	kzg: &KZG,
	values: &[Vec<u8>],
	commitment: &KZGCommitment,
	position: &Position,
) -> Option<Segment> {
	values
		.iter()
		.filter_map(|value| {
			// Attempt to decode the value into a SegmentData
			if let std::result::Result::Ok(segment_data) = SegmentData::decode(&mut &value[..]) {
				let segment = Segment { position: position.clone(), content: segment_data };
				// Safely check the segment and verify it
				if let std::result::Result::Ok(checked_segment) = segment.checked() {
					if let std::result::Result::Ok(vd) =
						checked_segment.verify(kzg, commitment, SEGMENTS_PER_BLOB)
					{
						if vd {
							return Some(segment)
						}
					}
				}
			}
			None
		})
		// Only return the segment that matches the provided position
		.find(|segment| segment.position == *position)
}

#[cfg(test)]
mod tests {
	use super::*;
	use codec::Encode;
	use melo_das_primitives::Blob;
	use melo_erasure_coding::{bytes_to_blobs, bytes_to_segments};
	use rand::Rng;

	fn random_bytes(len: usize) -> Vec<u8> {
		let mut rng = rand::thread_rng();
		let bytes: Vec<u8> = (0..len).map(|_| rng.gen()).collect();
		bytes
	}

	fn create_commitments(blobs: &[Blob]) -> Option<Vec<KZGCommitment>> {
		blobs
			.iter()
			.map(|blob| blob.commit(&KZG::default_embedded()))
			.collect::<Result<Vec<_>, _>>()
			.ok()
	}

	#[test]
	fn test_verify_values_valid() {
		let bytes = random_bytes(500);
		let segments = bytes_to_segments(
			&bytes,
			FIELD_ELEMENTS_PER_BLOB,
			FIELD_ELEMENTS_PER_SEGMENT,
			&KZG::default_embedded(),
		)
		.unwrap();

		let blobs = bytes_to_blobs(&bytes, FIELD_ELEMENTS_PER_BLOB).unwrap();

		let commitments = blobs
			.iter()
			.map(|blob| blob.commit(&KZG::default_embedded()))
			.collect::<Vec<_>>();

		let commitment = commitments[0].clone().unwrap();

		let segment = &segments[0];

		let segment_data_vec = vec![
			segments[2].content.clone().encode(),
			segment.content.clone().encode(),
			segments[1].content.clone().encode(),
		];

		let segment_option = verify_values(
			&KZG::default_embedded(),
			&segment_data_vec,
			&commitment,
			&segment.position,
		);

		assert!(segment_option.is_some());
		assert_eq!(segment_option.unwrap().content, segment.content.clone());

		let segment_data_vec =
			vec![segments[2].content.clone().encode(), segments[1].content.clone().encode()];
		let segment_option = verify_values(
			&KZG::default_embedded(),
			&segment_data_vec,
			&commitment,
			&segment.position,
		);
		assert!(segment_option.is_none());
	}

	#[test]
	fn test_verify_values_invalid() {
		let bytes = random_bytes(500);
		let segments = bytes_to_segments(
			&bytes,
			FIELD_ELEMENTS_PER_BLOB,
			FIELD_ELEMENTS_PER_SEGMENT,
			&KZG::default_embedded(),
		)
		.unwrap();

		let blobs = bytes_to_blobs(&bytes, FIELD_ELEMENTS_PER_BLOB).unwrap();

		let commitments = blobs
			.iter()
			.map(|blob| blob.commit(&KZG::default_embedded()))
			.collect::<Vec<_>>();

		let commitment = commitments[0].clone().unwrap();

		// Provide an invalid segment position.
		let invalid_position = Position { x: 9999, y: 9999 };

		let segment_data_vec =
			vec![segments[2].content.clone().encode(), segments[1].content.clone().encode()];

		let segment_option = verify_values(
			&KZG::default_embedded(),
			&segment_data_vec,
			&commitment,
			&invalid_position,
		);
		assert!(segment_option.is_none());

		// Provide a random segment data which should not match the commitment.
		let random_segment_data = random_bytes(100); // assuming 100 bytes is the size of a segment data
		let segment_option = verify_values(
			&KZG::default_embedded(),
			&[random_segment_data],
			&commitment,
			&segments[0].position,
		);
		assert!(segment_option.is_none());
	}

	#[test]
	fn test_values_set_handler() {
		// Setup your test data with all valid values
		let data = random_bytes(31 * FIELD_ELEMENTS_PER_BLOB);
		let blobs = bytes_to_blobs(&data, FIELD_ELEMENTS_PER_BLOB).unwrap();
		let commitments = create_commitments(&blobs).unwrap();

		let kzg = KZG::default_embedded();

		let segments =
			bytes_to_segments(&data, FIELD_ELEMENTS_PER_BLOB, FIELD_ELEMENTS_PER_SEGMENT, &kzg)
				.unwrap();

		assert_eq!(segments.len(), EXTENDED_SEGMENTS_PER_BLOB);

		let values_set: Vec<Option<Vec<Vec<u8>>>> = segments
			.iter()
			.map(|segment| {
				let encoded_segments = segment.content.encode();
				Some(vec![encoded_segments])
			})
			.collect::<Vec<_>>();

		let result = values_set_handler(&values_set, &commitments, &kzg);

		assert!(result.is_ok());

		let (segments_res, need_reconstruct, is_availability) = result.unwrap();

		assert_eq!(segments.len(), segments_res.len());

		for (segment, segment_res) in segments.iter().zip(segments_res.iter()) {
			assert_eq!(segment, segment_res.as_ref().unwrap());
		}

		assert_eq!(Some(segments[0].clone()), segments_res[0]);
		assert_eq!(need_reconstruct.len(), 0);
		assert!(is_availability);

		for (segment, segment_res) in segments.iter().zip(segments_res.iter()) {
			assert_eq!(segment, segment_res.as_ref().unwrap());
		}

		let mut values_set: Vec<Option<Vec<Vec<u8>>>> = values_set;
		values_set[0] = None;

		let result = values_set_handler(&values_set, &commitments, &kzg);

		assert!(result.is_ok());

		let (segments_res, need_reconstruct, is_availability) = result.unwrap();

		assert_eq!(need_reconstruct.len(), 1);
		assert_eq!(need_reconstruct[0], 0);
		assert_eq!(segments_res[0], None);

		assert!(is_availability);

		for i in 0..SEGMENTS_PER_BLOB + 2 {
			values_set[i] = None;
		}

		let result = values_set_handler(&values_set, &commitments, &kzg);

		assert!(result.is_ok());

		let (_, _, is_availability) = result.unwrap();

		assert!(!is_availability);
	}

	#[test]
	fn test_values_set_handler_empty() {
		let commitments: Vec<KZGCommitment> = vec![];
		let kzg = KZG::default_embedded();
		let values_set: Vec<Option<Vec<Vec<u8>>>> = vec![];

		let result = values_set_handler(&values_set, &commitments, &kzg);
		assert!(result.is_ok());
		let (_, need_reconstruct, is_availability) = result.unwrap();

		assert!(need_reconstruct.is_empty());
		assert!(!is_availability);
	}

	#[test]
	fn test_values_set_handler_unavailability() {
		let data = random_bytes(31 * FIELD_ELEMENTS_PER_BLOB * 5);
		let blobs = bytes_to_blobs(&data, FIELD_ELEMENTS_PER_BLOB).unwrap();
		let commitments = create_commitments(&blobs).unwrap();

		let kzg = KZG::default_embedded();

		let segments =
			bytes_to_segments(&data, FIELD_ELEMENTS_PER_BLOB, FIELD_ELEMENTS_PER_SEGMENT, &kzg)
				.unwrap();

		let mut values_set: Vec<Option<Vec<Vec<u8>>>> = segments
			.iter()
			.map(|segment| {
				let encoded_segments = segment.content.encode();
				Some(vec![encoded_segments])
			})
			.collect::<Vec<_>>();

		for i in 0..SEGMENTS_PER_BLOB - 1 {
			values_set[i] = None;
		}

		let result = values_set_handler(&values_set, &commitments, &kzg);
		assert!(result.is_ok());
		let (_, _, is_availability) = result.unwrap();

		assert!(is_availability);

		for i in SEGMENTS_PER_BLOB..SEGMENTS_PER_BLOB + 5 {
			values_set[i] = None;
		}

		let result = values_set_handler(&values_set, &commitments, &kzg);
		assert!(result.is_ok());
		let (_, _, is_availability) = result.unwrap();

		assert!(!is_availability);
	}
}

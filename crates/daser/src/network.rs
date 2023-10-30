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
use std::error::Error;

use crate::{
	sample_key, sample_key_from_block, Arc, KZGCommitment, Position, Sample, Segment, SegmentData,
	SAMPLES_PER_BLOB,
};
use melo_core_primitives::{traits::ExtendedHeader, Decode};
use melo_das_network::{KademliaKey, Service as DasNetworkService};
use melo_das_primitives::KZG;
use melo_erasure_coding::{
	extend_col::extend_segments_col as extend,
	recovery::recovery_order_row_from_segments as recovery,
};
use sp_api::HeaderT;

#[async_trait::async_trait]
pub trait DaserNetworker {
	async fn put_ext_segments<Header>(
		&self,
		segments: &[Segment],
		header: &Header,
	) -> Result<(), Box<dyn std::error::Error>>
	where
		Header: HeaderT;

	async fn put_app_segments(
		&self,
		segments: &[Segment],
		app_id: u32,
		nonce: u32,
	) -> Result<(), Box<dyn std::error::Error>>;

	async fn put_bytes(
		&self,
		bytes: &[u8],
		app_id: u32,
		nonce: u32,
	) -> Result<(), Box<dyn std::error::Error>>;

	async fn fetch_segment_data(
		&self,
		app_id: u32,
		nonce: u32,
		position: &Position,
		commitment: &KZGCommitment,
	) -> Option<SegmentData>;

	async fn fetch_sample(
		&self,
		app_id: u32,
		nonce: u32,
		sample: &Sample,
		commitment: &KZGCommitment,
	) -> Option<SegmentData>;

	async fn fetch_block<Header>(
		&self,
		header: &Header,
	) -> Result<(Vec<Option<Segment>>, Vec<usize>, bool), Box<dyn Error>>
	where
		Header: ExtendedHeader + HeaderT;

	fn extend_segments_col(&self, segments: &Vec<Segment>) -> Result<Vec<Segment>, String>;

	fn recovery_order_row_from_segments(
		&self,
		segments: &Vec<Option<Segment>>,
	) -> Result<Vec<Segment>, String>;
}

pub struct NetworkDas {
	network: Arc<DasNetworkService>,
	pub kzg: Arc<KZG>,
}

impl NetworkDas {
	pub fn new(network: Arc<DasNetworkService>, kzg: Arc<KZG>) -> Self {
		NetworkDas { network, kzg }
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

	pub fn prepare_keys<Header>(&self, header: &Header) -> Result<Vec<KademliaKey>, Box<dyn Error>>
	where
		Header: ExtendedHeader + HeaderT,
	{
		let keys = header
			.extension()
			.app_lookup
			.iter()
			.flat_map(|app_lookup| {
				(0..SAMPLES_PER_BLOB).flat_map(move |x| {
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
		values
			.iter()
			.filter_map(|value| {
				if let Ok(segment_data) = SegmentData::decode(&mut &value[..]) {
					let segment = Segment { position: position.clone(), content: segment_data };
					if segment
						.checked()
						.unwrap()
						.verify(&self.kzg, &commitment, segment.size())
						.is_ok()
					{
						return Some(segment)
					}
				}
				None
			})
			.next()
	}
}

#[async_trait::async_trait]
impl DaserNetworker for NetworkDas {
	fn extend_segments_col(&self, segments: &Vec<Segment>) -> Result<Vec<Segment>, String> {
		extend(&self.kzg.get_fs(), segments)
	}

	fn recovery_order_row_from_segments(
		&self,
		segments: &Vec<Option<Segment>>,
	) -> Result<Vec<Segment>, String> {
		recovery(segments, &self.kzg)
	}

	async fn put_ext_segments<Header>(
		&self,
		segments: &[Segment],
		header: &Header,
	) -> Result<(), Box<dyn std::error::Error>>
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

	async fn put_app_segments(
		&self,
		segments: &[Segment],
		app_id: u32,
		nonce: u32,
	) -> Result<(), Box<dyn std::error::Error>> {
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

	async fn put_bytes(
		&self,
		bytes: &[u8],
		app_id: u32,
		nonce: u32,
	) -> Result<(), Box<dyn std::error::Error>> {
		let segments = bytes_to_segments(bytes, SAMPLES_PER_BLOB, &self.kzg)?;
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
		app_id: u32,
		nonce: u32,
		sample: &Sample,
		commitment: &KZGCommitment,
	) -> Option<SegmentData> {
		let key = sample.key(app_id, nonce);
		self.fetch_value(&key, &sample.position, commitment).await
	}

	async fn fetch_block<Header>(
		&self,
		header: &Header,
	) -> Result<(Vec<Option<Segment>>, Vec<usize>, bool), Box<dyn Error>>
	where
		Header: ExtendedHeader + HeaderT,
	{
		let commitments =
			header.commitments().ok_or_else(|| "Header does not contain commitments.")?;
		let keys = self.prepare_keys(header)?;

		let values_set = self.network.get_values(&keys).await?;

		let mut need_reconstruct = vec![];
		let mut is_availability = true;

		let all_segments: Vec<Option<Segment>> = values_set
			.into_iter()
			.chunks(SAMPLES_PER_BLOB)
			.into_iter()
			.enumerate()
			.flat_map(|(y, chunk)| {
				if !is_availability {
					return vec![None; SAMPLES_PER_BLOB]
				}

				let segments = chunk
					.enumerate()
					.map(|(x, values)| match values {
						Some(values) => self.verify_values(
							&values,
							&commitments[y],
							&Position { x: x as u32, y: y as u32 },
						),
						None => None,
					})
					.collect::<Vec<_>>();

				if segments.iter().filter(|s| s.is_some()).count() > SAMPLES_PER_BLOB / 2 {
					need_reconstruct.push(y);
				} else {
					is_availability = false;
				}
				segments
			})
			.collect();

		Ok((all_segments, need_reconstruct, is_availability))
	}
}

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

//! Das Network Wrapper
//!
//! This module contains the DasNetworkServiceWrapper struct which wraps the DasNetworkService. It
//! provides methods for fetching values, preparing keys, and verifying values.
use codec::Encode;
use melo_erasure_coding::{bytes_to_segments, erasure_coding::extend_and_reorder_elements};

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

/// Defines a trait for network operations required by the DAS protocol.
#[async_trait::async_trait]
pub trait DasNetworkOperations {
	/// Puts external segments into the DAS network.
	///
	/// # Arguments
	///
	/// * `segments` - A slice of `Segment` to be put into the network.
	/// * `header` - A reference to the header of the segments.
	///
	/// # Type parameters
	///
	/// * `Header` - A type that implements `HeaderT`.
	///
	/// # Returns
	///
	/// Returns a `Result` indicating success or failure.
	async fn put_ext_segments<Header>(&self, segments: &[Segment], header: &Header) -> Result<()>
	where
		Header: HeaderT;

	/// Puts application segments into the DAS network.
	///
	/// # Arguments
	///
	/// * `segments` - A slice of `Segment` to be put into the network.
	/// * `app_id` - The ID of the application.
	/// * `nonce` - A nonce value.
	///
	/// # Returns
	///
	/// Returns a `Result` indicating success or failure.
	async fn put_app_segments(&self, segments: &[Segment], app_id: u32, nonce: u32) -> Result<()>;

	/// Puts bytes into the DAS network.
	///
	/// # Arguments
	///
	/// * `bytes` - A slice of bytes to be put into the network.
	/// * `app_id` - The ID of the application.
	/// * `nonce` - A nonce value.
	///
	/// # Returns
	///
	/// Returns a `Result` indicating success or failure.
	async fn put_bytes(&self, bytes: &[u8], app_id: u32, nonce: u32) -> Result<()>;

	/// Fetches segment data from the DAS network.
	///
	/// # Arguments
	///
	/// * `app_id` - The ID of the application.
	/// * `nonce` - A nonce value.
	/// * `position` - A reference to the position of the segment.
	/// * `commitment` - A reference to the KZG commitment.
	///
	/// # Returns
	///
	/// Returns an `Option` containing the fetched `SegmentData` or `None` if the data is not found.
	async fn fetch_segment_data(
		&self,
		app_id: u32,
		nonce: u32,
		position: &Position,
		commitment: &KZGCommitment,
	) -> Option<SegmentData>;

	/// Fetches a sample from the DAS network.
	///
	/// # Arguments
	///
	/// * `sample` - A reference to the sample to be fetched.
	/// * `commitment` - A reference to the KZG commitment.
	///
	/// # Returns
	///
	/// Returns an `Option` containing the fetched `SegmentData` or `None` if the data is not found.
	async fn fetch_sample(
		&self,
		sample: &Sample,
		commitment: &KZGCommitment,
	) -> Option<SegmentData>;

	/// Fetches a block from the DAS network.
	///
	/// # Arguments
	///
	/// * `header` - A reference to the header of the block.
	///
	/// # Type parameters
	///
	/// * `Header` - A type that implements `HeaderWithCommitment` and `HeaderT`.
	async fn fetch_block<Header>(&self, header: &Header) -> Result<(Vec<Option<Segment>>, bool)>
	where
		Header: HeaderWithCommitment + HeaderT;

	/// Extends the columns of the segments.
	///
	/// # Arguments
	///
	/// * `segments` - A slice of `Segment` to be extended.
	///
	/// # Returns
	///
	/// Returns a `Result` containing the extended `Segment`s.
	fn extend_segments_col(&self, segments: &[Segment]) -> Result<Vec<Segment>>;

	/// Recovers the order row from the segments.
	///
	/// # Arguments
	///
	/// * `segments` - A slice of `Option<Segment>` to recover the order row from.
	///
	/// # Returns
	///
	/// Returns a `Result` containing the recovered `Segment`s.
	fn recovery_order_row_from_segments(
		&self,
		segments: &[Option<Segment>],
	) -> Result<Vec<Segment>>;

	/// Returns a reference to the KZG instance.
	fn kzg(&self) -> Arc<KZG>;

	/// Removes records from the DAS network.
	///
	/// # Arguments
	///
	/// * `keys` - A vector of byte slices representing the keys of the records to be removed.
	///
	/// # Returns
	///
	/// Returns a `Result` indicating success or failure.
	async fn remove_records(&self, keys: Vec<&[u8]>) -> Result<()>;

	/// Fetches row segments based on the specified header and index.
	///
	/// This function is asynchronous and retrieves row segments associated with a given header.
	/// The segments are identified by the provided row indices. The function returns a tuple
	/// containing the fetched segments, indices of rows that need reconstruction, and a boolean
	/// flag indicating if all required segments are available.
	///
	/// # Arguments
	/// * `header` - A reference to the header associated with the segments. This header must
	///   implement `HeaderWithCommitment` and `HeaderT` traits.
	/// * `index` - A slice of `u32` representing the indices of the rows to fetch.
	///
	/// # Returns
	/// A `Result` containing:
	/// * A `Vec<Option<Segment>>` where each element represents a segment at the corresponding
	///   index in `index`. If a segment is not available, the corresponding element is `None`.
	/// * A `bool` flag indicating if all required segments are available (true if available).
	async fn fetch_rows<Header>(
		&self,
		header: &Header,
		index: &[u32],
	) -> Result<(Vec<Option<Segment>>, bool)>
	where
		Header: HeaderWithCommitment + std::marker::Sync;

	/// Fetches column segments based on the specified header and index.
	///
	/// Similar to `fetch_rows`, this asynchronous function retrieves column segments associated
	/// with a given header. The segments are identified by the provided column indices. It returns
	/// a tuple with the fetched segments, indices of columns that need reconstruction, and a
	/// boolean flag indicating if all required segments are available.
	///
	/// # Arguments
	/// * `header` - A reference to the header associated with the segments. The header must
	///   implement `HeaderWithCommitment` and `HeaderT` traits.
	/// * `index` - A slice of `u32` representing the indices of the columns to fetch.
	///
	/// # Returns
	/// A `Result` containing:
	/// * A `Vec<Option<Segment>>` where each element corresponds to a segment at the respective
	///   index in `index`. Elements are `None` if the segment is unavailable.
	/// * A `Vec<usize>` containing the indices of columns that require reconstruction.
	/// * A `bool` indicating whether all necessary segments are available (true if available).
	///
	/// # Errors
	/// This function may return an error if fetching the segments encounters problems, such as
	/// network issues or if the required segments are not available.
	async fn fetch_cols<Header>(
		&self,
		header: &Header,
		index: &[u32],
	) -> Result<(Vec<Option<Segment>>, Vec<usize>, bool)>
	where
		Header: HeaderWithCommitment + std::marker::Sync;
}

/// DasNetworkServiceWrapper is a struct that wraps the DasNetworkService and KZG structs.
/// It provides methods for fetching values, preparing keys, and verifying values.
#[derive(Clone, Debug)]
pub struct DasNetworkServiceWrapper {
	network: Arc<DasNetworkService>,
	/// The KZG instance.
	pub kzg: Arc<KZG>,
}

impl DasNetworkServiceWrapper {
	/// Creates a new instance of DasNetworkServiceWrapper.
	pub fn new(network: Arc<DasNetworkService>, kzg: Arc<KZG>) -> Self {
		DasNetworkServiceWrapper { network, kzg }
	}

	/// Fetches a segment of data from the network.
	async fn fetch_value(
		&self,
		key: &[u8],
		position: &Position,
		commitment: &KZGCommitment,
	) -> Option<SegmentData> {
		let values = self.network.get_value(KademliaKey::new(&key)).await.ok()?;
		self.verify_values(&values, commitment, position).map(|segment| segment.content)
	}

	/// Prepares keys for a given header.
	pub fn prepare_keys<Header>(&self, header: &Header) -> Result<Vec<KademliaKey>>
	where
		Header: HeaderWithCommitment,
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

	pub fn prepare_rows_keys<Header>(
		&self,
		header: &Header,
		index: &[u32],
		row_count: u32,
	) -> Result<Vec<KademliaKey>>
	where
		Header: HeaderWithCommitment,
	{
		if index.is_empty() {
			return Ok(vec![])
		}

		let extension = header.extension();

		let mut keys = Vec::new();
		for &idx in index {
			if idx < row_count {
				for x in 0..EXTENDED_SEGMENTS_PER_BLOB {
					let position = Position { x: x as u32, y: idx };
					let at = idx * EXTENDED_SEGMENTS_PER_BLOB as u32 + x as u32;

					if let Some((app_lookup, _)) = extension.get_lookup(at) {
						let key = sample_key(app_lookup.app_id, app_lookup.nonce, &position);
						keys.push(KademliaKey::new(&key));
					} else {
						return Err(anyhow!("prepare_rows_keys: get_lookup failed"))
					}
				}
			} else {
				if idx >= row_count * 2 {
					return Err(anyhow!("prepare_rows_keys: idx is too large"))
				}
				let block_hash = HeaderWithCommitment::hash(header);
				for x in 0..EXTENDED_SEGMENTS_PER_BLOB {
					let position = Position { x: x as u32, y: idx };
					let key = sample_key_from_block(&block_hash.encode(), &position);
					keys.push(KademliaKey::new(&key));
				}
			}
		}

		Ok(keys)
	}

	pub fn prepare_cols_keys<Header>(
		&self,
		header: &Header,
		index: &[u32],
		row_count: u32,
	) -> Result<Vec<KademliaKey>>
	where
		Header: HeaderWithCommitment,
	{
		if index.is_empty() {
			return Ok(vec![])
		}

		let block_hash = HeaderWithCommitment::hash(header);
		let extension = header.extension();
		let mut keys = Vec::new();

		for &idx in index {
			for y in 0..row_count {
				let position = Position { x: idx, y };
				let at = idx * row_count * 2 + y;
				let app_lookup = extension
					.get_lookup(at)
					.ok_or_else(|| anyhow!("prepare_cols_keys: get_lookup failed for sample_key"))?
					.0;
				let key = sample_key(app_lookup.app_id, app_lookup.nonce, &position);
				keys.push(KademliaKey::new(&key));
			}

			for y in row_count..(row_count * 2) {
				let position = Position { x: idx, y };
				let key = sample_key_from_block(&block_hash.encode(), &position);
				keys.push(KademliaKey::new(&key));
			}
		}

		Ok(keys)
	}

	/// Verifies the values of a segment.
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

	async fn fetch_block<Header>(&self, header: &Header) -> Result<(Vec<Option<Segment>>, bool)>
	where
		Header: HeaderWithCommitment + std::marker::Sync,
	{
		let commitments = header.commitments().context("Header does not contain commitments.")?;
		let keys = self.prepare_keys(header)?;

		let values_set = self.network.get_values(&keys).await?;

		rows_values_set_handler(&values_set, &commitments, &self.kzg, true)
	}

	async fn fetch_rows<Header>(
		&self,
		header: &Header,
		index: &[u32],
	) -> Result<(Vec<Option<Segment>>, bool)>
	where
		Header: HeaderWithCommitment + std::marker::Sync,
	{
		let commitments = header.commitments().context("Header does not contain commitments.")?;

		let commits_exted =
			extend_and_reorder_elements(self.kzg.get_fs(), &commitments).map_err(|e| anyhow!(e))?;
		let keys = self.prepare_rows_keys(header, index, commitments.len() as u32)?;

		let values_set = self.network.get_values(&keys).await?;

		rows_values_set_handler(&values_set, &commits_exted, &self.kzg, false)
	}

	/// Fetches columns of data from the network.
	async fn fetch_cols<Header>(
		&self,
		header: &Header,
		index: &[u32],
	) -> Result<(Vec<Option<Segment>>, Vec<usize>, bool)>
	where
		Header: HeaderWithCommitment + std::marker::Sync,
	{
		let commitments = header.commitments().context("Header does not contain commitments.")?;

		let commits_exted =
			extend_and_reorder_elements(self.kzg.get_fs(), &commitments).map_err(|e| anyhow!(e))?;
		let keys = self.prepare_cols_keys(header, index, commitments.len() as u32)?;

		let values_set = self.network.get_values(&keys).await?;

		// Custom handler for column-based data
		cols_values_set_handler(&values_set, &commits_exted, &self.kzg, false)
	}

	async fn remove_records(&self, keys: Vec<&[u8]>) -> Result<()> {
		let keys = keys.into_iter().map(|key| KademliaKey::new(&key)).collect::<Vec<_>>();
		self.network.remove_records(&keys).await
	}
}

fn cols_values_set_handler(
	values_set: &[Option<Vec<Vec<u8>>>],
	commitments: &[KZGCommitment],
	kzg: &KZG,
	stop_on_unavailability: bool,
) -> Result<(Vec<Option<Segment>>, Vec<usize>, bool)> {
	if values_set.is_empty() || commitments.is_empty() {
		return Ok((vec![], vec![], false))
	}

	let mut need_reconstruct = vec![];
	let mut is_availability = true;

	let segments_per_col = commitments.len();
	let segments_per_col_exted = segments_per_col * 2;

	let all_segments: Vec<Option<Segment>> = values_set
		.chunks(segments_per_col_exted)
		.enumerate()
		.flat_map(|(x, column_values)| {
			if !is_availability && stop_on_unavailability {
				return vec![None; EXTENDED_SEGMENTS_PER_BLOB]
			}

			let segments = column_values
				.iter()
				.enumerate()
				.map(|(y, values)| match values {
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

			if available_segments >= segments_per_col {
				if available_segments < segments_per_col_exted {
					need_reconstruct.push(x);
				}
			} else {
				is_availability = false;
			}
			segments
		})
		.collect();

	Ok((all_segments, need_reconstruct, is_availability))
}

fn rows_values_set_handler(
	values_set: &[Option<Vec<Vec<u8>>>],
	commitments: &[KZGCommitment],
	kzg: &KZG,
	stop_on_unavailability: bool,
) -> Result<(Vec<Option<Segment>>, bool)> {
	if values_set.is_empty() || commitments.is_empty() {
		return Ok((vec![], true))
	}

	let mut is_availability = true;

	let all_segments: Vec<Option<Segment>> = values_set
		.chunks(EXTENDED_SEGMENTS_PER_BLOB)
		.enumerate()
		.flat_map(|(y, chunk)| {
			if !is_availability && stop_on_unavailability {
				return vec![None; EXTENDED_SEGMENTS_PER_BLOB]
			}
			let (segments, availability) = rows_values_handler(kzg, chunk, y, &commitments[y]);
			is_availability = is_availability && availability;
			segments
		})
		.collect();

	Ok((all_segments, is_availability))
}

fn rows_values_handler(
	kzg: &KZG,
	row: &[Option<Vec<Vec<u8>>>],
	y: usize,
	commitment: &KZGCommitment,
) -> (Vec<Option<Segment>>, bool) {
	let (segments, some_count) = row.iter().enumerate().fold(
		(Vec::new(), 0),
		|(mut acc, count), (x, values)| match values {
			Some(values) => {
				acc.push(verify_values(
					kzg,
					values,
					commitment,
					&Position { x: x as u32, y: y as u32 },
				));
				(acc, count + 1)
			},
			None => {
				acc.push(None);
				(acc, count)
			},
		},
	);

	let needs_recovery = some_count > segments.len() / 2 && some_count < segments.len();
	let mut is_availability = some_count >= segments.len() / 2;

	if needs_recovery {
		let segments = recovery(&segments, kzg)
			.map(|recovered_segments| recovered_segments.into_iter().map(Some).collect())
			.unwrap_or_else(|_| {
				is_availability = false;
				segments
			});
		(segments, is_availability)
	} else {
		(segments, is_availability)
	}
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
	fn test_rows_values_set_handler() {
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

		let result = rows_values_set_handler(&values_set, &commitments, &kzg, true);

		assert!(result.is_ok());

		let (segments_res, is_availability) = result.unwrap();

		assert_eq!(segments.len(), segments_res.len());

		for (segment, segment_res) in segments.iter().zip(segments_res.iter()) {
			assert_eq!(segment, segment_res.as_ref().unwrap());
		}

		assert_eq!(Some(segments[0].clone()), segments_res[0]);
		assert!(is_availability);

		for (segment, segment_res) in segments.iter().zip(segments_res.iter()) {
			assert_eq!(segment, segment_res.as_ref().unwrap());
		}

		let mut values_set: Vec<Option<Vec<Vec<u8>>>> = values_set;
		values_set[0] = None;

		let result = rows_values_set_handler(&values_set, &commitments, &kzg, true);

		assert!(result.is_ok());

		let (segments_res, is_availability) = result.unwrap();

		assert_eq!(segments_res[0], None);

		assert!(is_availability);

		for i in 0..SEGMENTS_PER_BLOB + 2 {
			values_set[i] = None;
		}

		let result = rows_values_set_handler(&values_set, &commitments, &kzg, true);

		assert!(result.is_ok());

		let (_, is_availability) = result.unwrap();

		assert!(!is_availability);
	}

	#[test]
	fn test_rows_values_set_handler_empty() {
		let commitments: Vec<KZGCommitment> = vec![];
		let kzg = KZG::default_embedded();
		let values_set: Vec<Option<Vec<Vec<u8>>>> = vec![];

		let result = rows_values_set_handler(&values_set, &commitments, &kzg, true);
		assert!(result.is_ok());
		let (_, is_availability) = result.unwrap();

		assert!(!is_availability);
	}

	#[test]
	fn test_rows_values_set_handler_unavailability() {
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

		let result = rows_values_set_handler(&values_set, &commitments, &kzg, true);
		assert!(result.is_ok());
		let (_, is_availability) = result.unwrap();

		assert!(is_availability);

		for i in SEGMENTS_PER_BLOB..SEGMENTS_PER_BLOB + 5 {
			values_set[i] = None;
		}

		let result = rows_values_set_handler(&values_set, &commitments, &kzg, true);
		assert!(result.is_ok());
		let (_, is_availability) = result.unwrap();

		assert!(!is_availability);
	}
}

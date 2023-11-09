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

//! Melo Erasure Coding
//! 
//! This crate provides functions for erasure coding and recovery of data.
use crate::{
	erasure_coding::{extend_poly, recover_poly},
	segment::{order_segments_row, segment_datas_to_row},
};
use melo_das_primitives::{
	crypto::{Position, KZG},
	segment::{Segment, SegmentData},
};
use rust_kzg_blst::utils::reverse_bit_order;

use crate::{String, ToString, Vec};

/// Recover the segment datas from the given segment datas, KZG, chunk count, y, and segments size.
/// 
/// # Arguments
/// 
/// * `segment_datas` - A slice of optional segment data.
/// * `kzg` - A reference to a KZG instance.
/// * `chunk_count` - The number of chunks.
/// * `y` - The y coordinate.
/// * `segments_size` - The size of the segments.
/// 
/// # Returns
/// 
/// A Result containing a vector of segments or an error message as a string.
pub fn recover_segment_datas(
	segment_datas: &[Option<SegmentData>],
	kzg: &KZG,
	chunk_count: usize,
	y: u32,
	segments_size: usize,
) -> Result<Vec<Segment>, String> {
	let mut row = segment_datas_to_row(segment_datas, segments_size);
	reverse_bit_order(&mut row);
	let poly = recover_poly(kzg.get_fs(), &row)?;

	let recovery_row = extend_poly(kzg.get_fs(), &poly)?;

	segment_datas
		.iter()
		.enumerate()
		.map(|(i, segment_data)| {
			let position = Position { x: i as u32, y };
			match segment_data {
				Some(segment_data) => Ok(Segment { position, content: segment_data.clone() }),
				None => {
					let index = i * segments_size;
					let data = recovery_row[index..(index + segments_size)].to_vec();
					let segment_data =
						SegmentData::from_data(&position, &data, kzg, &poly, chunk_count)?;
					Ok(Segment { position, content: segment_data })
				},
			}
		})
		.collect()
}

/// Recover a row of segments from a vector of segments, using the provided KZG instance and chunk
/// count.
///
/// # Arguments
///
/// * `segments` - A vector of `Segment`s to recover a row from.
/// * `kzg` - A `KZG` instance to use for recovery.
/// * `chunk_count` - The number of segments in the original data.
pub fn recovery_row_from_segments(
	segments: &Vec<Segment>,
	kzg: &KZG,
	chunk_count: usize,
) -> Result<Vec<Segment>, String> {
	let y = segments[0].position.y;
	let segments_size = segments[0].size();

	if segments.iter().any(|s| s.position.y != y) {
		return Err("segments are not from the same row".to_string())
	}
	if !segments_size.is_power_of_two() || !chunk_count.is_power_of_two() {
		return Err("segment size and chunk_count must be a power of two".to_string())
	}
	if segments.iter().any(|s| s.size() != segments_size) {
		return Err("segments are not of the same size".to_string())
	}

	let order_segments = order_segments_row(segments, chunk_count)?;
	recover_segment_datas(
		&order_segments.iter().map(|s| s.as_ref().cloned()).collect::<Vec<_>>(),
		kzg,
		chunk_count,
		y,
		segments_size,
	)
}

/// Given a slice of `Option<Segment>`s, where each `Segment` represents a chunk of data, this function returns a vector 
/// of `Segment`s that represent the recovered data of the same row. The function uses the provided `KZG` object to recover 
/// the data. 
/// 
/// # Arguments
/// 
/// * `order_segments` - A slice of `Option<Segment>`s, where each `Segment` represents a chunk of data.
/// * `kzg` - A `KZG` object used to recover the data.
/// 
/// # Returns
/// 
/// A `Result` containing a vector of `Segment`s that represent the recovered data of the same row, or an error message if the 
/// segment size and chunk count are not a power of two, or if the segments are not from the same row or not of the same size.
pub fn recovery_order_row_from_segments(
	order_segments: &[Option<Segment>],
	kzg: &KZG,
) -> Result<Vec<Segment>, String> {
	let chunk_count = order_segments.len();
	let mut y = None;
	let mut size = None;

	if !chunk_count.is_power_of_two() {
		return Err("segment size and chunk_count must be a power of two".to_string())
	}

	if order_segments.iter().any(|s| {
		if let Some(segment) = s {
			if y.is_none() {
				y = Some(segment.position.y);
			}
			if size.is_none() {
				size = Some(segment.size());
			}
			y == Some(segment.position.y) && size == Some(segment.size())
		} else {
			true
		}
	}) {
		return Err("segments are not from the same row or not of the same size".to_string())
	}

	let segment_datas = order_segments
		.iter()
		.map(|s| s.as_ref().map(|segment| segment.content.clone()))
		.collect::<Vec<_>>();

	let segments_size =
		size.ok_or_else(|| "Error: Failed to determine the size of segments.".to_string())?;
	let y = y.ok_or_else(|| "Error: Failed to determine the row position (y).".to_string())?;

	recover_segment_datas(&segment_datas, kzg, chunk_count, y, segments_size)
}

// TODO
// pub fn recovery_col_from_segments(kzg: &KZG, segments: &Vec<Segment>, k: usize) ->
// Result<Vec<Segment>, String> {}

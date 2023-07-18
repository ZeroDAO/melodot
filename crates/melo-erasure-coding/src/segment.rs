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
use melo_core_primitives::config::{FIELD_ELEMENTS_PER_BLOB, SEGMENT_LENGTH};
use melo_core_primitives::kzg::BlsScalar;
use melo_core_primitives::segment::{Segment, SegmentData};

pub fn order_segments_row(segments: &Vec<Segment>) -> Result<Vec<Option<SegmentData>>, String> {
	if segments.len() > FIELD_ELEMENTS_PER_BLOB * 2 {
		return Err("segments x not equal".to_string());
	}
	let y = segments[0].position.y;
	let mut ordered_segments = vec![None; FIELD_ELEMENTS_PER_BLOB * 2];
	for segment in segments.iter() {
		if segment.position.y != y {
			return Err("segments y not equal".to_string());
		}
		ordered_segments[segment.position.x as usize] = Some(segment.content.clone());
	}
	Ok(ordered_segments)
}

pub fn order_segments_col(
	segments: &Vec<Segment>,
	k: usize,
) -> Result<Vec<Option<SegmentData>>, String> {
	if segments.len() > k * 2 {
		return Err("segments x not equal".to_string());
	}
	let x = segments[0].position.x;
	let mut ordered_segments = vec![None; k * 2];
	for segment in segments.iter() {
		if segment.position.x != x {
			return Err("segments x not equal".to_string());
		}
		ordered_segments[segment.position.x as usize] = Some(segment.content.clone());
	}
	Ok(ordered_segments)
}

pub fn segment_datas_to_row(segments: &Vec<Option<SegmentData>>) -> Vec<Option<BlsScalar>> {
	segments
		.iter()
		.flat_map(|segment_data_option| match segment_data_option {
			Some(segment_data) => segment_data
				.data
				.iter()
				.map(|scalar| Some(*scalar))
				.collect::<Vec<Option<BlsScalar>>>(),
			None => vec![None; SEGMENT_LENGTH],
		})
		.collect::<Vec<Option<BlsScalar>>>()
}

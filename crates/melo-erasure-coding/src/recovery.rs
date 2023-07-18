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
use crate::segment::{order_segments_row, segment_datas_to_row};
use melo_core_primitives::kzg::{Position, KZG};
use melo_core_primitives::segment::{Segment, SegmentData};

pub fn recovery_row_from_segments(
	segments: &Vec<Segment>,
	kzg: &KZG,
) -> Result<Vec<Segment>, String> {
	let y = segments[0].position.y;
	let order_segments = order_segments_row(&segments)?;
	let row = segment_datas_to_row(&order_segments);
	let poly = kzg.recover_poly(&row)?;
	let recovery_row = poly.to_bls_scalars();
	order_segments
		.iter()
		.enumerate()
		.map(|(i, segment_data)| {
			let position = Position { x: i as u32, y };
			match segment_data {
				Some(segment_data) => Ok(Segment { position, content: segment_data.clone() }),
				None => {
					let index = i * SegmentData::SIZE;
					let data = recovery_row[index..(i + 1) * SegmentData::SIZE].to_vec();
					let segment_data = SegmentData::from_data(&position, &data, kzg, &poly).map_err(|e| e.to_string())?;
                    Ok(Segment { position, content: segment_data })
				},
			}
		})
		.collect()
}

// pub fn recovery_column_from_segments(segments: &Vec<Segment>, kzg: &KZG) -> Vec<Segment> {
// }

// pub fn recovery_row_from_cells(segments: &Vec<Cell>, kzg: &KZG) -> Vec<Cell> {
// }

// pub fn recovery_column_from_cells(segments: &Vec<Cell>, kzg: &KZG) -> Vec<Cell> {
// }
// #[path = "./local_tests/local_consts.rs"]
// pub mod local_consts;

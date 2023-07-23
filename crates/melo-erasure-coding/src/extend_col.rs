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

#![cfg_attr(not(feature = "std"), no_std)]
use itertools::Itertools;
use melo_core_primitives::{
	kzg::BlsScalar,
	segment::{Segment, SegmentData},
};
use rust_kzg_blst::types::fft_settings::FsFFTSettings;

use crate::erasure_coding::{extend, extend_fs_g1};

pub fn extend_segments_col(
	fs: &FsFFTSettings,
	segments: &Vec<Segment>,
) -> Result<Vec<Segment>, String> {
	let k = segments.len();
	let x = segments[0].position.x;

	let mut proofs = vec![];
	let sorted_rows: Vec<BlsScalar> = segments
		.iter()
		.sorted_by_key(|s| s.position.y)
		.enumerate()
		.filter(|(i, s)| s.position.x == x && *i == s.position.y as usize)
		.flat_map(|(_, s)| {
			proofs.push(s.content.proof.clone());
			s.content.data.clone()
		})
		.collect();

	if sorted_rows.len() != k * Segment::SIZE {
		return Err("segments x not equal".to_string());
	}

	let extended_proofs = extend_fs_g1(fs, &proofs)?;

	let mut extended_cols = vec![];

	for i in 0..(Segment::SIZE) {
		let col: Vec<BlsScalar> = sorted_rows
			.iter()
			.skip(i)
			.step_by(Segment::SIZE as usize)
			.map(|s| s.clone())
			.collect::<Vec<BlsScalar>>();
		extended_cols.push(extend(fs, &col)?);
	}

	let mut extended_segments = vec![];

	// 需要获取奇数部分
	extended_proofs.iter().skip(1).step_by(2).enumerate().for_each(|(i, proof)| {
		let position = melo_core_primitives::kzg::Position { x, y: (i + k) as u32 };
		let data = extended_cols.iter().map(|col| col[i]).collect::<Vec<BlsScalar>>();
		let segment = Segment { position, content: SegmentData { data, proof: proof.clone() } };
		extended_segments.push(segment);
	});

	Ok(extended_segments)
}
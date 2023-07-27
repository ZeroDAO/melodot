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

/// Extends the segments in a column using FFT settings.
/// 
/// It extends the `segments` in the original column to twice their size, and also extends the `proof` in each 
/// `Segment`.The homomorphic property of KZG commitments is used to extend the proof to the correct commitment of the 
/// row where the data is located. This avoids the cost of recalculating commitments and proofs.
///
/// # Arguments
///
/// * `fs` - FFT settings to use for the extension.
/// * `segments` - Segments to extend. The number of segments must be a power of two.
///
/// # Returns
///
/// * `Result<Vec<Segment>, String>` - A vector of extended segments, or an error message if the extension fails.
/// 
/// # Notes
/// 
/// * The extended `Vec<Segment>` is not interleaved with parity data, and the `y` value of the `Position` in the original 
/// data is not changed. This is to avoid confusion during the erasure coding process.
pub fn extend_segments_col(
    fs: &FsFFTSettings,
    segments: &Vec<Segment>,
) -> Result<Vec<Segment>, String> {
    let k = segments.len();
    let x = segments[0].position.x;
    let segment_size = segments[0].size();

    // Check if all segments are from the same column
    if segments.iter().any(|s| s.position.x != x) {
        return Err("segments are not from the same column".to_string());
    }

    // Check if k and segment_size are powers of two
    if !k.is_power_of_two() || !segment_size.is_power_of_two() {
        return Err("number of segments and segment size must be powers of two".to_string());
    }

    let mut proofs = vec![];
    let sorted_rows: Vec<BlsScalar> = segments
        .iter()
        .sorted_by_key(|s| s.position.y)
        .enumerate()
        .filter(|(i, s)| s.position.x == x && *i == s.position.y as usize && s.size() == segment_size)
        .flat_map(|(_, s)| {
            proofs.push(s.content.proof.clone());
            s.content.data.clone()
        })
        .collect();

    // Check if the number of elements after sorting is equal to k * segment_size
    if sorted_rows.len() != k * segment_size {
        return Err("mismatch in the number of elements after sorting".to_string());
    }

    // Extend the proofs using FFT
    let extended_proofs = extend_fs_g1(fs, &proofs)?;

    let mut extended_cols = vec![];

    // Extend each column using FFT
    for i in 0..(segment_size) {
        let col: Vec<BlsScalar> = sorted_rows
            .iter()
            .skip(i)
            .step_by(segment_size)
            .map(|s| s.clone())
            .collect::<Vec<BlsScalar>>();
        extended_cols.push(extend(fs, &col)?);
    }

    let mut extended_segments = vec![];

    // Obtain the odd parts of the extended proofs and create new segments
    extended_proofs.iter().skip(1).step_by(2).enumerate().for_each(|(i, proof)| {
        let position = melo_core_primitives::kzg::Position { x, y: (i + k) as u32 };
        let data = extended_cols.iter().map(|col| col[i]).collect::<Vec<BlsScalar>>();
        let segment = Segment { position, content: SegmentData { data, proof: proof.clone() } };
        extended_segments.push(segment);
    });

    Ok(extended_segments)
}
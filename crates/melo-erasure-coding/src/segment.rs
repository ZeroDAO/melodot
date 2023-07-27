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
use kzg::FK20MultiSettings;
use melo_core_primitives::kzg::{BlsScalar, KZGProof, Polynomial, Position, KZG};
use melo_core_primitives::segment::{Segment, SegmentData};
use rust_kzg_blst::types::fk20_multi_settings::FsFK20MultiSettings;

use crate::erasure_coding::extend_poly;

/// Orders a vector of `Segment`s into a row of `SegmentData` using the provided chunk count.
/// 
/// The returned vector is of type `Vec<Option<SegmentData>>`, where `Option<SegmentData>` is an `Option` type. 
/// If there is data at a position, it is `Some`, otherwise it is `None`.The final length of the returned vector 
/// is `chunk_count * 2`, where `chunk_count` is the length of the original data.
/// 
/// # Arguments
/// 
/// * `segments` - A vector of `Segment`s to order into a row.
/// * `chunk_count` - The number of chunks to use for ordering.
/// 
/// # Returns
/// 
/// A `Result` containing a vector of `Option<SegmentData>` representing the ordered row, or an error message 
/// if ordering fails.
pub fn order_segments_row(segments: &Vec<Segment>, chunk_count: usize) -> Result<Vec<Option<SegmentData>>, String> {
    if segments.len() > chunk_count * 2 || segments.len() == 0 {
        return Err("segments x not equal".to_string());
    }
    let y = segments[0].position.y;
    let mut ordered_segments = vec![None; chunk_count * 2];
    for segment in segments.iter() {
        if segment.position.y != y {
            return Err("segments y not equal".to_string());
        }
        ordered_segments[segment.position.x as usize] = Some(segment.content.clone());
    }
    Ok(ordered_segments)
}

/// Orders segments by column.
/// 
/// The returned vector is of type `Vec<Option<SegmentData>>`, where `Option<SegmentData>` is an `Option` type.
/// If there is data at a position, it is `Some`, otherwise it is `None`. The final length of the returned vector
/// is `k * 2`, where `k` is the number of segments in the original data.
///
/// # Arguments
///
/// * `segments` - A vector of `Segment` structs to be ordered.
/// * `k` - The number of segments to be ordered.
///
/// # Returns
///
/// A `Result` containing a vector of `Option<SegmentData>` structs if successful, or a `String` error message if unsuccessful.
///
/// # Errors
///
/// Returns an error message if the length of `segments` is greater than `k * 2` or if the length of `segments` is 0.
pub fn order_segments_col(
    segments: &Vec<Segment>,
    k: usize,
) -> Result<Vec<Option<SegmentData>>, String> {
    if segments.len() > k * 2 || segments.len() == 0 {
        return Err("segments x not equal".to_string());
    }
    let x = segments[0].position.x;
    let mut ordered_segments = vec![None; k * 2];
    for segment in segments.iter() {
        if segment.position.x != x {
            return Err("segments x not equal".to_string());
        }
        ordered_segments[segment.position.y as usize] = Some(segment.content.clone());
    }
    Ok(ordered_segments)
}

/// Converts a vector of `SegmentData` structs to a vector of `BlsScalar` structs.
///
/// # Arguments
///
/// * `segments` - A reference to a vector of `Option<SegmentData>` structs to be converted. The length 
/// of the vector must be a power of two.
/// * `chunk_size` - The size of each chunk. Must be a power of two.
///
/// # Returns
///
/// A vector of `Option<BlsScalar>` structs.
pub fn segment_datas_to_row(segments: &Vec<Option<SegmentData>>, chunk_size: usize) -> Vec<Option<BlsScalar>> {
    segments
        .iter()
        .flat_map(|segment_data_option| match segment_data_option {
            Some(segment_data) => segment_data
                .data
                .iter()
                .map(|scalar| Some(*scalar))
                .collect::<Vec<Option<BlsScalar>>>(),
            None => vec![None; chunk_size],
        })
        .collect::<Vec<Option<BlsScalar>>>()
}

/// Converts a polynomial to a vector of segments.
///
/// # Arguments
///
/// * `poly` - A reference to a `Polynomial` struct to be converted. The length of the polynomial must be a power of two.
/// * `kzg` - A reference to a `KZG` struct.
/// * `y` - The y-coordinate of the position of the segment.
/// * `chunk_size` - The size of each chunk. Must be a power of two.
///
/// # Returns
///
/// A `Result` containing a vector of `Segment` structs if successful, or a `String` error message if unsuccessful.
///
/// # Errors
///
/// Returns an error message if `chunk_size` is not a power of two.
pub fn poly_to_segment_vec(poly: &Polynomial, kzg: &KZG, y: usize, chunk_size: usize) -> Result<Vec<Segment>, String> {
    let poly_len = poly.checked()?.0.coeffs.len();

    // chunk_size must be a power of two
    if !chunk_size.is_power_of_two() {
        return Err("chunk_size must be a power of two".to_string());
    }

    let fk = FsFK20MultiSettings::new(&kzg.ks, 2 * poly_len, chunk_size).unwrap();
    let all_proofs = fk.data_availability(&poly.0).unwrap();
    let extended_poly = extend_poly(&fk.kzg_settings.fs, &poly)?;
    let segments = extended_poly
        .chunks(chunk_size)
        .enumerate()
        .map(|(i, chunk)| {
            let position = Position { y: y as u32, x: i as u32 };
            let proof = all_proofs[i];
            Segment::new(position, chunk, KZGProof(proof))
        })
        .collect::<Vec<_>>();

    Ok(segments)
}

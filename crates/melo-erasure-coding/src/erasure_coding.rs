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

use kzg::{FFTFr, Fr, PolyRecover, DAS, FFTG1, G1};
use melo_das_primitives::{
	crypto::{BlsScalar, ReprConvert},
	polynomial::Polynomial,
};

use rust_kzg_blst::{
	types::{fft_settings::FsFFTSettings, fr::FsFr, g1::FsG1, poly::FsPoly},
	utils::reverse_bit_order,
};

use crate::{String, Vec};

/// Extends the given `source` slice using the provided `FsFFTSettings`.
///
/// It will return an parity data number of elements of the same length because it will use the FFT
/// to expand the input to 2n elements.
///
/// # Arguments
///
/// * `fs` - A reference to an `FsFFTSettings` instance.
/// * `source` - A slice of `BlsScalar` to extend. The length of the slice must be a power of two.
///
/// # Returns
///
/// Returns a `Result` containing a `Vec` of extended `BlsScalar` instances or an error message.
pub fn extend(fs: &FsFFTSettings, source: &[BlsScalar]) -> Result<Vec<BlsScalar>, String> {
	fs.das_fft_extension(BlsScalar::slice_to_repr(source))
		.map(BlsScalar::vec_from_repr)
}

/// Extends the given `Polynomial` instance using the provided `FsFFTSettings`.
///
/// It returns data that has been extended to twice its length and has been processed with
/// `reverse_bit_order`, making it directly applicable to data sampling.
///
/// # Arguments
///
/// * `fs` - A reference to an `FsFFTSettings` instance.
/// * `poly` - A reference to a `Polynomial` instance to extend.
///
/// # Returns
///
/// Returns a `Result` containing a `Vec` of extended `BlsScalar` instances or an error message.
pub fn extend_poly(fs: &FsFFTSettings, poly: &Polynomial) -> Result<Vec<BlsScalar>, String> {
	let mut coeffs = poly.0.coeffs.clone();
	coeffs.resize(coeffs.len() * 2, FsFr::zero());
	let mut extended_coeffs_fft = fs.fft_fr(&coeffs, false)?;
	reverse_bit_order(&mut extended_coeffs_fft);
	Ok(BlsScalar::vec_from_repr(extended_coeffs_fft))
}

/// Extends the given slice of `T` instances using the provided `FsFFTSettings` and `FsG1` scalar
/// field.
///
/// This is used to extend data for `KZGCommitment` and `KZGProof`, both of which are of type
/// `FsG1`. It returns a result that interleaves the original data with parity data, so column
/// extensions need to be handled separately.
///
/// # Arguments
///
/// * `fs` - A reference to an `FsFFTSettings` instance.
/// * `source` - A slice of `T` instances to extend.
///
/// # Returns
///
/// Returns a `Result` containing a `Vec` of extended `T` instances or an error message.
pub fn extend_fs_g1<T: ReprConvert<FsG1>>(
	fs: &FsFFTSettings,
	source: &[T],
) -> Result<Vec<T>, String> {
	let mut coeffs = fs.fft_g1(T::slice_to_repr(source), true)?;
	coeffs.resize(coeffs.len() * 2, FsG1::identity());
	fs.fft_g1(&coeffs, false).map(T::vec_from_repr)
}

/// Extends an array of elements by doubling its size using FFT and then reorders
/// the extended array such that original elements are followed by the extended elements.
///
/// # Arguments
///
/// * `fs`: Reference to the `FsFFTSettings` which contains the FFT settings.
/// * `elements`: A slice of type `T` which implements `ReprConvert<FsG1>`. This represents the
///   original elements to be extended and reordered.
///
/// # Returns
///
/// This function returns a `Result` which, on success, contains a `Vec<T>` representing
/// the reordered elements with the original data followed by the extended data.
/// On failure, it returns a `String` describing the error.
///
/// # Errors
///
/// This function will return an error if the extension and reordering of elements fails,
/// typically due to issues within the FFT process or invalid input data.
pub fn extend_and_reorder_elements<T: ReprConvert<FsG1>>(
	fs: &FsFFTSettings,
	elements: &[T],
) -> Result<Vec<T>, String> {
	// Extend the elements to double their size using FFT.
	let extended = extend_fs_g1(fs, elements)?;

	// Split the extended data into two vectors: one for original and one for extended elements.
	// Original data is at even indices, and extended data is at odd indices.
	let (originals, extensions): (Vec<_>, Vec<_>) =
		extended.into_iter().enumerate().partition(|&(i, _)| i % 2 == 0);

	// Reorder the elements by concatenating the original and extended data.
	Ok(originals
		.into_iter()
		.map(|(_, e)| e)
		.chain(extensions.into_iter().map(|(_, e)| e))
		.collect())
}

/// Recovers the original data from the given `shards` slice using the provided `FsFFTSettings`.
///
/// The `shards` slice should contain more than half of the valid data, otherwise it will not be
/// possible to recover the original data and an error will be returned. If the `shards` slice
/// contains `None`, the function will return the original data.
///
/// # Arguments
///
/// * `fs` - A reference to an `FsFFTSettings` instance.
/// * `shards` - A slice of `Option<BlsScalar>` instances to recover data from.
///
/// # Returns
///
/// Returns a `Result` containing a `Vec` of recovered `BlsScalar` instances or an error message.
pub fn recover(fs: &FsFFTSettings, shards: &[Option<BlsScalar>]) -> Result<Vec<BlsScalar>, String> {
	match shards.contains(&None) {
		true => {
			let mut shards_mut = shards.to_vec();
			reverse_bit_order(&mut shards_mut);
			let mut poly =
				FsPoly::recover_poly_from_samples(BlsScalar::slice_option_to_repr(&shards_mut), fs)?;
			reverse_bit_order(&mut poly.coeffs);
			Ok(BlsScalar::vec_from_repr(poly.coeffs))
		},
		false => {
			// shards does not contain None, it is safe to unwrap
			Ok(shards.iter().map(|s| s.unwrap()).collect::<Vec<BlsScalar>>())
		},
	}
}

/// Recovers a polynomial from the given shards using the provided FFT settings.
///
/// It checks if `shards` contains no `None` values, and if so, directly computes the polynomial
/// using FFT. This also prevents errors when `shards` contains no `None` values.
///
/// # Arguments
///
/// * `fs` - FFT settings to use for the recovery.
/// * `shards` - Shards to recover the polynomial from.
pub fn recover_poly(
	fs: &FsFFTSettings,
	shards: &[Option<BlsScalar>],
) -> Result<Polynomial, String> {
	let mut shards_mut = shards.to_vec();
	reverse_bit_order(&mut shards_mut);

	let mut poly = match shards.contains(&None) {
		true => {
			let poly = FsPoly::recover_poly_coeffs_from_samples(
				BlsScalar::slice_option_to_repr(&shards_mut),
				fs,
			)?;
			Polynomial::from(poly)
		},
		false => {
			// shards does not contain None, it is safe
			let data = shards_mut.iter().map(|s| s.unwrap()).collect::<Vec<BlsScalar>>();
			let coeffs = fs.fft_fr(BlsScalar::slice_to_repr(&data), true).expect("");
			Polynomial::from_coeffs(&coeffs)
		},
	};

	poly.left();

	Ok(poly)
}

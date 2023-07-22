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
use kzg::{FFTFr,Fr, PolyRecover, DAS, FFTG1, G1};
use melo_core_primitives::kzg::BlsScalar;
use melo_core_primitives::kzg::ReprConvert;
use melo_core_primitives::kzg::Blob;
use melo_core_primitives::kzg::Polynomial;

use rust_kzg_blst::{
	types::{
		fft_settings::FsFFTSettings, fr::FsFr, g1::FsG1,
		poly::FsPoly,
	},
	utils::reverse_bit_order,
};

pub fn poly(fs: &FsFFTSettings, blob: &Blob) -> Result<FsPoly, String> {
    // self.new_fft_settings(data.len())?;
    let poly = FsPoly { coeffs: fs.fft_fr(&blob.0, true)? };
    Ok(poly)
}

pub fn extend(fs: &FsFFTSettings, source: &[BlsScalar]) -> Result<Vec<BlsScalar>, String> {
    fs
        .das_fft_extension(BlsScalar::slice_to_repr(source))
        .map(BlsScalar::vec_from_repr)
}

pub fn extend_poly(fs: &FsFFTSettings,  poly: &Polynomial) -> Result<Vec<FsFr>, String> {
    let mut coeffs = poly.0.coeffs.clone();
    coeffs.resize(coeffs.len() * 2, FsFr::zero());
    let mut extended_coeffs_fft = fs.fft_fr(&coeffs, false).unwrap();
    reverse_bit_order(&mut extended_coeffs_fft);
    Ok(extended_coeffs_fft)
}

pub fn extend_fs_g1<T: ReprConvert<FsG1>>(fs: &FsFFTSettings, source: &[T]) -> Result<Vec<T>, String> {
    let mut coeffs = fs.fft_g1(T::slice_to_repr(source), true)?;

    coeffs.resize(coeffs.len() * 2, FsG1::identity());

    fs.fft_g1(&coeffs, false).map(T::vec_from_repr)
}

pub fn recover(fs: &FsFFTSettings, shards: &[Option<BlsScalar>]) -> Result<Vec<BlsScalar>, String> {
    let poly = FsPoly::recover_poly_from_samples(
        BlsScalar::slice_option_to_repr(shards),
        &fs,
    )?;

    Ok(BlsScalar::vec_from_repr(poly.coeffs))
}

pub fn recover_poly(fs: &FsFFTSettings, shards: &[Option<BlsScalar>]) -> Result<Polynomial, String> {
    let poly =
        FsPoly::recover_poly_from_samples(BlsScalar::slice_option_to_repr(shards), &fs)?;
    Ok(Polynomial::from(poly))
}
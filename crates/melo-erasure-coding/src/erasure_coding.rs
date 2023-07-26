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
use kzg::{FFTFr, Fr, PolyRecover, DAS, FFTG1, G1};
use melo_core_primitives::kzg::BlsScalar;
use melo_core_primitives::kzg::Polynomial;
use melo_core_primitives::kzg::ReprConvert;

use rust_kzg_blst::{
	types::{fft_settings::FsFFTSettings, fr::FsFr, g1::FsG1, poly::FsPoly},
	utils::reverse_bit_order,
};

pub fn extend(fs: &FsFFTSettings, source: &[BlsScalar]) -> Result<Vec<BlsScalar>, String> {
	fs.das_fft_extension(BlsScalar::slice_to_repr(source))
		.map(BlsScalar::vec_from_repr)
}

pub fn extend_poly(fs: &FsFFTSettings, poly: &Polynomial) -> Result<Vec<BlsScalar>, String> {
	let mut coeffs = poly.0.coeffs.clone();
	coeffs.resize(coeffs.len() * 2, FsFr::zero());
	let mut extended_coeffs_fft = fs.fft_fr(&coeffs, false).unwrap();
	reverse_bit_order(&mut extended_coeffs_fft);
	Ok(BlsScalar::vec_from_repr(extended_coeffs_fft))
}

pub fn extend_fs_g1<T: ReprConvert<FsG1>>(
	fs: &FsFFTSettings,
	source: &[T],
) -> Result<Vec<T>, String> {
	let mut coeffs = fs.fft_g1(T::slice_to_repr(source), true)?;
	coeffs.resize(coeffs.len() * 2, FsG1::identity());
	fs.fft_g1(&coeffs, false).map(T::vec_from_repr)
}

pub fn recover(fs: &FsFFTSettings, shards: &[Option<BlsScalar>]) -> Result<Vec<BlsScalar>, String> {
	match shards.contains(&None) {
		true => {
			let poly = FsPoly::recover_poly_from_samples(BlsScalar::slice_option_to_repr(shards), &fs)?;
			Ok(BlsScalar::vec_from_repr(poly.coeffs))
		},
		false => {
			// shards 中不包含 None ，它是安全的
			Ok(shards.iter().map(|s| s.unwrap()).collect::<Vec<BlsScalar>>())
		},
	}
}

pub fn recover_poly(
	fs: &FsFFTSettings,
	shards: &[Option<BlsScalar>],
) -> Result<Polynomial, String> {
	// 检查 shards 前半部分是否没有 none ，如果没有，则使用快速傅里叶直接计算
	// 同时也是防止整个shares 没有 none 的情况下，引起错误
	
	let mut poly = match shards.contains(&None) {
		true => {
			let poly = FsPoly::recover_poly_coeffs_from_samples(BlsScalar::slice_option_to_repr(shards), &fs)?;
			Polynomial::from(poly)
		},
		false => {
			// shards 中不包含 None ，它是安全的
			let data = shards.iter().map(|s| s.unwrap()).collect::<Vec<BlsScalar>>();
			let coeffs = fs.fft_fr(BlsScalar::slice_to_repr(&data), true).expect("");
			Polynomial::from_coeffs(&coeffs)
		},
	};

	poly.left();

	Ok(poly)
}
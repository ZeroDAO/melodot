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

use derive_more::From;
use kzg::{FFTFr, Poly};
use rust_kzg_blst::{
	types::{fft_settings::FsFFTSettings, fr::FsFr, poly::FsPoly},
	utils::reverse_bit_order,
};

use crate::crypto::{BlsScalar, ReprConvert};
use crate::Blob;
use alloc::{
	string::{String, ToString},
	vec::Vec,
};

/// A polynomial represented by a `FsPoly` struct.
#[derive(Debug, Clone, From)]
pub struct Polynomial(pub FsPoly);

impl Polynomial {
	/// Creates a new polynomial with the given size.
	///
	/// # Arguments
	///
	/// * `size` - The size of the polynomial.
	///
	/// # Returns
	///
	/// A `Result` containing the new polynomial or an error message.
	pub fn new(size: usize) -> Result<Self, String> {
		FsPoly::new(size).map(Self)
	}

	/// Checks if the polynomial is valid.
	///
	/// It checks if the number of coefficients is a power of two to validate the polynomial.
	fn is_valid(&self) -> bool {
		self.0.coeffs.len().is_power_of_two()
	}

	/// Checks if the polynomial is valid and returns a new polynomial.
	///
	/// If valid, returns the original polynomial. If invalid, returns an error.
	// TODO We should not clone()
	pub fn checked(&self) -> Result<Self, String> {
		if !self.is_valid() {
			return Err("Polynomial size must be a power of two".to_string());
		}
		Ok(self.clone())
	}

	/// Creates a new polynomial from the given coefficients.
	pub fn from_coeffs(coeffs: &[FsFr]) -> Self {
		Polynomial(FsPoly { coeffs: coeffs.to_vec() })
	}

	/// Truncates the polynomial to the left half.
	///
	/// This is an in-place operation that truncates the coefficients of the polynomial to the
	/// left half. This is typically used when restoring polynomial coefficients, where the left
	/// half is usually returned and the right half is empty. Therefore, it is necessary to ensure
	/// that the correct result should be returned before truncating.
	pub fn left(&mut self) {
		let half = self.0.coeffs.len() / 2;
		self.0.coeffs.truncate(half);
	}

	/// Converts the polynomial to a slice of `BlsScalar` values.
	pub fn to_bls_scalars(&self) -> &[BlsScalar] {
		BlsScalar::slice_from_repr(&self.0.coeffs)
	}

	/// Converts the polynomial to a `Blob`.
	///
	/// This directly converts the polynomial data to a Blob in Lagrange form.
	pub fn to_blob(&self) -> Blob {
		Blob::from(self.to_bls_scalars().to_vec())
	}

	/// Evaluates the polynomial at all points.
	///
	/// The data in the polynomial is used as coefficients, and FFT transformation is performed.
	/// The result is given in Lagrange form on twice the length. The returned result is a `BlsScalar` array,
	/// twice the length of the polynomial, and has already undergone `reverse_bit_order`.
	pub fn eval_all(&self, fs: &FsFFTSettings) -> Result<Vec<BlsScalar>, String> {
		let mut reconstructed_data = fs.fft_fr(&self.0.coeffs, false)?;
		reverse_bit_order(&mut reconstructed_data);
		Ok(BlsScalar::vec_from_repr(reconstructed_data))
	}

	/// Evaluates the polynomial at the given point.
	pub fn eval(&self, x: &BlsScalar) -> BlsScalar {
		BlsScalar(self.0.eval(x))
	}
}

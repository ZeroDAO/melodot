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

use alloc::{
	string::{String, ToString},
	sync::Arc,
	vec::Vec,
};
use core::hash::{Hash, Hasher};
use core::mem;
use core::ptr;
use derive_more::{AsMut, AsRef, Deref, DerefMut, From, Into};
use kzg::eip_4844::{BYTES_PER_G1, BYTES_PER_G2};
use kzg::{FFTSettings, FK20MultiSettings, Fr, KZGSettings, G1, G2};
use parity_scale_codec::{Decode, Encode, EncodeLike, Input, MaxEncodedLen};

use rust_kzg_blst::types::{
	fft_settings::FsFFTSettings, fk20_multi_settings::FsFK20MultiSettings, fr::FsFr, g1::FsG1,
	g2::FsG2, kzg_settings::FsKZGSettings,
};
use scale_info::{Type, TypeInfo};

use crate::{
	blob::Blob,
	config::{BYTES_PER_FIELD_ELEMENT, EMBEDDED_KZG_SETTINGS_BYTES},
	polynomial::Polynomial,
};

// kzg_type_with_size macro are inspired by
// https://github.com/subspace/subspace/blob/main/crates/subspace-core-primitives/src/crypto/kzg.rs
// but we use macros instead of implementing them separately for each type.
macro_rules! kzg_type_with_size {
	($name:ident, $type:ty, $size:expr) => {
		#[derive(
			Debug, Default, Copy, Clone, PartialEq, Eq, Into, From, AsRef, AsMut, Deref, DerefMut,
		)]
		#[repr(transparent)]
		pub struct $name(pub $type);
		impl $name {
			#[warn(dead_code)]
			const SIZE: usize = $size;

			#[inline]
			pub fn to_bytes(&self) -> [u8; $size] {
				self.0.to_bytes()
			}

			#[inline]
			pub fn try_from_bytes(bytes: &[u8; $size]) -> Result<Self, String> {
				Ok($name(<$type>::from_bytes(bytes)?))
			}
		}

		impl Hash for $name {
			fn hash<H: Hasher>(&self, state: &mut H) {
				self.to_bytes().hash(state);
			}
		}

		impl From<$name> for [u8; $size] {
			#[inline]
			fn from(kzd_data: $name) -> Self {
				kzd_data.to_bytes()
			}
		}

		impl From<&$name> for [u8; $size] {
			#[inline]
			fn from(kzd_data: &$name) -> Self {
				kzd_data.to_bytes()
			}
		}

		impl TryFrom<&[u8; $size]> for $name {
			type Error = String;

			#[inline]
			fn try_from(bytes: &[u8; $size]) -> Result<Self, Self::Error> {
				Self::try_from_bytes(bytes)
			}
		}

		impl TryFrom<[u8; $size]> for $name {
			type Error = String;

			#[inline]
			fn try_from(bytes: [u8; $size]) -> Result<Self, Self::Error> {
				Self::try_from(&bytes)
			}
		}

		impl Encode for $name {
			#[inline]
			fn size_hint(&self) -> usize {
				Self::SIZE
			}

			fn using_encoded<R, F: FnOnce(&[u8]) -> R>(&self, f: F) -> R {
				f(&self.to_bytes())
			}

			#[inline]
			fn encoded_size(&self) -> usize {
				Self::SIZE
			}
		}

		impl EncodeLike for $name {}

		impl MaxEncodedLen for $name {
			#[inline]
			fn max_encoded_len() -> usize {
				Self::SIZE
			}
		}

		impl Decode for $name {
			fn decode<I: Input>(input: &mut I) -> Result<Self, parity_scale_codec::Error> {
				Self::try_from_bytes(&Decode::decode(input)?).map_err(|error| {
					parity_scale_codec::Error::from("Failed to decode from bytes")
						.chain(alloc::format!("{error:?}"))
				})
			}

			#[inline]
			fn encoded_fixed_size() -> Option<usize> {
				Some(Self::SIZE)
			}
		}

		impl TypeInfo for $name {
			type Identity = Self;

			fn type_info() -> Type {
				Type::builder()
					.path(scale_info::Path::new(stringify!($name), module_path!()))
					.docs(&["Commitment to polynomial"])
					.composite(scale_info::build::Fields::named().field(|f| {
						f.ty::<[u8; $size]>().name(stringify!(inner)).type_name("G1Affine")
					}))
			}
		}
	};
}

// TODO: Automatic size reading
kzg_type_with_size!(KZGCommitment, FsG1, BYTES_PER_G1);
kzg_type_with_size!(KZGProof, FsG1, BYTES_PER_G1);
kzg_type_with_size!(BlsScalar, FsFr, BYTES_PER_FIELD_ELEMENT);

/// The `ReprConvert` trait defines methods for converting between types `Self` and `T`.
pub trait ReprConvert<T>: Sized {
	/// Convert a slice of type `Self` to a slice of type `T`.
	///
	/// # Safety
	/// This method uses `unsafe` code because it transmutes the pointer from `&[Self]` to `&[T]`.
	/// Calling this method requires ensuring that the conversion is safe and that `Self` and `T` have the same memory layout.
	fn slice_to_repr(value: &[Self]) -> &[T];

	/// Convert a slice of type `T` to a slice of type `Self`.
	///
	/// # Safety
	/// This method uses `unsafe` code because it transmutes the pointer from `&[T]` to `&[Self]`.
	/// Calling this method requires ensuring that the conversion is safe and that `Self` and `T` have the same memory layout.
	fn slice_from_repr(value: &[T]) -> &[Self];

	/// Convert a `Vec` of type `Self` to a `Vec` of type `T`.
	///
	/// # Safety
	/// This method uses `unsafe` code because it transmutes the pointer from `Vec<Self>` to `Vec<T>`.
	/// Calling this method requires ensuring that the conversion is safe and that `Self` and `T` have the same memory layout.
	fn vec_to_repr(value: Vec<Self>) -> Vec<T>;

	/// Convert a `Vec` of type `T` to a `Vec` of type `Self`.
	///
	/// # Safety
	/// This method uses `unsafe` code because it transmutes the pointer from `Vec<T>` to `Vec<Self>`.
	/// Calling this method requires ensuring that the conversion is safe and that `Self` and `T` have the same memory layout.
	fn vec_from_repr(value: Vec<T>) -> Vec<Self>;

	/// Convert a slice of `Option<Self>` to a slice of `Option<T>`.
	///
	/// # Safety
	/// This method uses `unsafe` code because it transmutes the pointer from `&[Option<Self>]` to `&[Option<T>]`.
	/// Calling this method requires ensuring that the conversion is safe and that `Self` and `T` have the same memory layout.
	fn slice_option_to_repr(value: &[Option<Self>]) -> &[Option<T>];
}

/// This macro provides a convenient way to convert a slice of the underlying representation to a
/// commitment for efficiency purposes. To ensure safe conversion, the #[repr(transparent)] attribute
/// must be implemented.
macro_rules! repr_convertible {
	($name:ident, $type:ty) => {
		impl ReprConvert<$type> for $name {
			#[inline]
			fn slice_to_repr(value: &[Self]) -> &[$type] {
				unsafe { &*(value as *const [Self] as *const [$type]) }
			}

			#[inline]
			fn slice_from_repr(value: &[$type]) -> &[Self] {
				unsafe { &*(value as *const [$type] as *const [Self]) }
			}

			#[inline]
			fn vec_to_repr(value: Vec<Self>) -> Vec<$type> {
				let mut value = mem::ManuallyDrop::new(value);
				unsafe {
					let ptr = value.as_mut_ptr() as *mut $type;
					Vec::from_raw_parts(ptr, value.len(), value.capacity())
				}
			}

			#[inline]
			fn vec_from_repr(value: Vec<$type>) -> Vec<Self> {
				let mut value = mem::ManuallyDrop::new(value);
				unsafe {
					let ptr = value.as_mut_ptr() as *mut Self;
					Vec::from_raw_parts(ptr, value.len(), value.capacity())
				}
			}

			#[inline]
			fn slice_option_to_repr(value: &[Option<Self>]) -> &[Option<$type>] {
				unsafe { &*(value as *const [Option<Self>] as *const [Option<$type>]) }
			}
		}
	};
}

// TODO: Change to automatic implementation
repr_convertible!(Blob, Vec<BlsScalar>);
repr_convertible!(KZGCommitment, FsG1);
repr_convertible!(KZGProof, FsG1);
repr_convertible!(BlsScalar, FsFr);

/// BlsScalar is 32 bytes, but we only use 31 bytes for safe operations
/// 32 bytes is not safe, because it can be greater than the modulus
/// https://github.com/supranational/blst/blob/327d30a51c858e9c34f5b6eb3a6966b2cf6bc9cc/src/exports.c#L107
pub trait SafeScalar: Sized {
	/// Safe size of scalar
	const SAFE_SIZE: usize = SCALAR_SAFE_BYTES;
	/// Safe size of scalar
	fn safe_size() -> usize;
	/// Try to convert bytes to scalar
	fn try_from_bytes_safe(bytes: &[u8; SCALAR_SAFE_BYTES]) -> Result<Self, String>;
	/// Convert scalar to bytes
	fn to_bytes_safe(&self) -> [u8; SCALAR_SAFE_BYTES];
}

/// BlsScalar is 32 bytes, but we only use 31 bytes for safe operations
pub const SCALAR_SAFE_BYTES: usize = 31;

impl SafeScalar for BlsScalar {
	const SAFE_SIZE: usize = SCALAR_SAFE_BYTES;

	fn safe_size() -> usize {
		Self::SAFE_SIZE
	}

	fn try_from_bytes_safe(value: &[u8; SCALAR_SAFE_BYTES]) -> Result<Self, String> {
		let v_ptr = value.as_ptr();
		let mut full_scalar = [0u8; Self::SIZE];
		let f_ptr = full_scalar.as_mut_ptr();

		// SCALAR_SAFE_BYTES is always less than FULL_SCALAR_BYTES, so it is safe.
		unsafe {
			ptr::copy_nonoverlapping(v_ptr, f_ptr, SCALAR_SAFE_BYTES);
		}

		Self::try_from(full_scalar)
	}

	fn to_bytes_safe(&self) -> [u8; SCALAR_SAFE_BYTES] {
		let full_bytes = self.to_bytes();
		let mut bytes = [0u8; SCALAR_SAFE_BYTES];

		unsafe {
			ptr::copy_nonoverlapping(full_bytes.as_ptr(), bytes.as_mut_ptr(), SCALAR_SAFE_BYTES);
		}

		bytes
	}
}

impl From<&[u8; SCALAR_SAFE_BYTES]> for BlsScalar {
	#[inline]
	fn from(value: &[u8; SCALAR_SAFE_BYTES]) -> Self {
		let v_ptr = value.as_ptr();
		let mut full_scalar = [0u8; Self::SIZE];
		let f_ptr = full_scalar.as_mut_ptr();

		// SCALAR_SAFE_BYTES is always less than FULL_SCALAR_BYTES, so it is safe.
		unsafe {
			ptr::copy_nonoverlapping(v_ptr, f_ptr, SCALAR_SAFE_BYTES);
		}

		Self::try_from(full_scalar)
			.expect("Safe bytes always fit into scalar and thus succeed; qed")
	}
}

impl From<[u8; SCALAR_SAFE_BYTES]> for BlsScalar {
	#[inline]
	fn from(value: [u8; SCALAR_SAFE_BYTES]) -> Self {
		Self::from(&value)
	}
}

/// Number of G1 powers stored in [`EMBEDDED_KZG_SETTINGS_BYTES`]
pub const NUM_G1_POWERS: usize = 4_096;
/// Number of G2 powers stored in [`EMBEDDED_KZG_SETTINGS_BYTES`]
pub const NUM_G2_POWERS: usize = 65;

// This function is derived and modified from `https://github.com/sifraitech/rust-kzg/blob/main/blst/src/eip_4844.rs#L75` .
pub fn bytes_to_kzg_settings(
	g1_bytes: &[u8],
	g2_bytes: &[u8],
	num_g1_powers: usize,
	num_g2_powers: usize,
) -> Result<FsKZGSettings, String> {
	let num_g1_points = g1_bytes.len() / BYTES_PER_G1;

	if num_g1_points != num_g1_powers || num_g2_powers != g2_bytes.len() / BYTES_PER_G2 {
		return Err("Invalid bytes length".to_string());
	}

    let g1_values = g1_bytes
        .chunks_exact(BYTES_PER_G1)
        .map(FsG1::from_bytes)
        .collect::<Result<Vec<_>, _>>()?;

	let g2_values = g2_bytes
        .chunks_exact(BYTES_PER_G2)
        .map(FsG2::from_bytes)
        .collect::<Result<Vec<_>, _>>()?;

	let fs = FsFFTSettings::new(
		num_g1_powers
			.checked_sub(1)
			.expect("Checked to be not empty above; qed")
			.ilog2() as usize,
	)
	.expect("Scale is within allowed bounds; qed");

	Ok(FsKZGSettings { secret_g1: g1_values, secret_g2: g2_values, fs })
}

/// Embedded KZG settings, currently using the trusted setup of Ethereum. You can generate the required data
/// using `scripts/process_data.sh`.
///
/// ```bash
/// ./scripts/process_data.sh 4096
/// ```
///
/// Changing `4096` will generate data of different lengths. There are several options: `["4096" "8192" "16384" "32768"]`.
// This references subpace's design https://github.com/subspace/subspace/blob/main/crates/subspace-core-primitives/src/crypto/kzg.rs#L101
// This will slightly increase the size of the compiled binary, but it can reduce complexity in the `no-std` 
// environment. In the long run, we still need to optimize it.
fn embedded_kzg_settings() -> FsKZGSettings {
	let (secret_g1_bytes, secret_g2_bytes) =
		EMBEDDED_KZG_SETTINGS_BYTES.split_at(BYTES_PER_G1 * NUM_G1_POWERS);
	bytes_to_kzg_settings(secret_g1_bytes, secret_g2_bytes, NUM_G1_POWERS, NUM_G2_POWERS)
		.expect("Static bytes are correct, there is a test for this; qed")
}

/// KZG is a struct that represents a KZG instance.
#[derive(Debug, Clone, AsMut)]
pub struct KZG {
	pub ks: Arc<FsKZGSettings>,
}

impl KZG {
	/// Create a new KZG instance with the given settings.
	pub fn new(kzg_settings: FsKZGSettings) -> Self {
		Self { ks: Arc::new(kzg_settings) }
	}

	/// Get the maximum width of the KZG instance.
	pub fn max_width(&self) -> usize {
		self.ks.fs.max_width
	}

	/// Create a new KZG instance with the embedded settings.
	pub fn default_embedded() -> Self {
		Self::new(embedded_kzg_settings())
	}

	/// Get the expanded roots of unity at the given index.
	pub fn get_expanded_roots_of_unity_at(&self, i: usize) -> FsFr {
		self.ks.get_expanded_roots_of_unity_at(i)
	}

	/// Get the KZG index for the given chunk count, chunk index, and chunk size.
	pub fn get_kzg_index(
		&self,
		chunk_count: usize,
		chunk_index: usize,
		chunk_size: usize,
	) -> usize {
		let domain_stride = self.max_width() / (2 * chunk_size * chunk_count);
		let domain_pos = Self::reverse_bits_limited(chunk_count, chunk_index);
		domain_pos * domain_stride
	}

	/// Compute all proofs for the given polynomial and chunk size.
	///
	/// # Arguments
	///
	/// * `poly` - The polynomial to compute proofs for.
	/// * `chunk_size` - The size of each chunk.
	///
	/// # Returns
	///
	/// A vector of KZGProofs, one for each chunk.
	pub fn all_proofs(
		&self,
		poly: &Polynomial,
		chunk_size: usize,
	) -> Result<Vec<KZGProof>, String> {
		let poly_len = poly.0.coeffs.len();
		let fk = FsFK20MultiSettings::new(&self.ks, 2 * poly_len, chunk_size).unwrap();
		let all_proofs = fk.data_availability(&poly.0).unwrap();
		Ok(KZGProof::vec_from_repr(all_proofs))
	}

	/// Compute a proof for the given polynomial, chunk index, count, and chunk size.
	///
	/// # Arguments
	///
	/// * `poly` - The polynomial to compute the proof for.
	/// * `chunk_index` - The index of the chunk to compute the proof for.
	/// * `count` - The total number of chunks.
	/// * `chunk_size` - The size of each chunk.
	///
	/// # Returns
	///
	/// A KZGProof for the given chunk.
	pub fn compute_proof_multi(
		&self,
		poly: &Polynomial,
		chunk_index: usize,
		count: usize,
		chunk_size: usize,
	) -> Result<KZGProof, String> {
		let pos = self.get_kzg_index(count, chunk_index, chunk_size);
		let x = self.get_expanded_roots_of_unity_at(pos);
		self.ks.compute_proof_multi(&poly.0, &x, chunk_size).map(KZGProof)
	}

	/// Check a proof for the given commitment, index, count, values, proof, and n.
	///
	/// # Arguments
	///
	/// * `commitment` - The KZGCommitment to check the proof against.
	/// * `i` - The index of the chunk to check the proof for.
	/// * `count` - The total number of chunks.
	/// * `chunk_count` - The values of the chunk to check the proof for.
	/// * `proof` - The KZGProof to check.
	/// * `chunk_size` - The size of each chunk.
	///
	/// # Returns
	///
	/// A boolean indicating whether the proof is valid.
	pub fn check_proof_multi(
		&self,
		commitment: &KZGCommitment,
		i: usize,
		chunk_count: usize,
		values: &[FsFr],
		proof: &KZGProof,
		chunk_size: usize,
	) -> Result<bool, String> {
		let pos = self.get_kzg_index(chunk_count, i, chunk_size);
		let x = self.get_expanded_roots_of_unity_at(pos);
		self.ks.check_proof_multi(&commitment.0, &proof.0, &x, values, chunk_size)
	}

	/// Compute a proof for the given polynomial and point index.
	///
	/// # Arguments
	///
	/// * `poly` - The polynomial to compute the proof for.
	/// * `i` - The index of the point to compute the proof for.
	///
	/// # Returns
	///
	/// A KZGProof for the given point.
	pub fn compute_proof_with_index(
		&self,
		poly: &Polynomial,
		i: usize,
	) -> Result<KZGProof, String> {
		let x = self.get_expanded_roots_of_unity_at(i);
		self.ks.compute_proof_single(&poly.0, &x).map(KZGProof)
	}

	/// Compute a proof for the given polynomial and x value.
	///
	/// # Arguments
	///
	/// * `poly` - The polynomial to compute the proof for.
	/// * `x` - The x value to compute the proof for.
	///
	/// # Returns
	///
	/// A KZGProof for the given x value.
	pub fn compute_proof(&self, poly: &Polynomial, x: &FsFr) -> Result<KZGProof, String> {
		self.ks.compute_proof_single(&poly.0, x).map(KZGProof)
	}

	/// Commit to the given polynomial.
	///
	/// # Arguments
	///
	/// * `poly` - The polynomial to commit to.
	///
	/// # Returns
	///
	/// A KZGCommitment for the given polynomial.
	pub fn commit(&self, poly: &Polynomial) -> Result<KZGCommitment, String> {
		self.ks.commit_to_poly(&poly.0).map(KZGCommitment)
	}

	/// Verify the given commitment, index, value, and proof.
	///
	/// # Arguments
	///
	/// * `commitment` - The KZGCommitment to verify.
	/// * `index` - The index of the point to verify.
	/// * `value` - The value of the point to verify.
	/// * `proof` - The KZGProof to verify.
	///
	/// # Returns
	///
	/// A boolean indicating whether the proof is valid.
	pub fn verify(
		&self,
		commitment: &KZGCommitment,
		index: u32,
		value: &BlsScalar,
		proof: &KZGProof,
	) -> Result<bool, String> {
		let x = self.get_expanded_roots_of_unity_at(index as usize);
		self.ks.check_proof_single(commitment, proof, &x, value)
	}

	/// Check a proof for the given commitment, proof, x, and value.
	///
	/// # Arguments
	///
	/// * `commitment` - The KZGCommitment to check the proof against.
	/// * `proof` - The KZGProof to check.
	/// * `x` - The x value to check the proof for.
	/// * `value` - The value to check the proof for.
	///
	/// # Returns
	///
	/// A boolean indicating whether the proof is valid.
	pub fn check_proof_single(
		&self,
		commitment: &KZGCommitment,
		proof: &KZGProof,
		x: &FsFr,
		value: &BlsScalar,
	) -> Result<bool, String> {
		self.ks.check_proof_single(commitment, proof, x, value)
	}

	/// Get the `FsFFTSettings` for the KZG instance.
	pub fn get_fs(&self) -> &FsFFTSettings {
		&self.ks.fs
	}

	/// Reverse the bits of the given value up to the given length.
	fn reverse_bits_limited(length: usize, value: usize) -> usize {
		let unused_bits = length.leading_zeros();
		value.reverse_bits() >> unused_bits
	}
}

#[derive(Debug, Default, Clone, PartialEq, Eq, From)]
pub struct Position {
	pub x: u32,
	pub y: u32,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, From, AsRef, AsMut)]
pub struct Cell {
	pub data: BlsScalar,
	pub position: Position,
}

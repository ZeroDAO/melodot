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
use kzg::{
	eip_4844::{BYTES_PER_G1, BYTES_PER_G2},
	FFTFr,
};
use kzg::{FFTSettings, FK20MultiSettings, Fr, KZGSettings, Poly, G1, G2};
use parity_scale_codec::{Decode, Encode, EncodeLike, Input, MaxEncodedLen};

use rust_kzg_blst::{
	types::{
		fft_settings::FsFFTSettings, fk20_multi_settings::FsFK20MultiSettings, fr::FsFr, g1::FsG1,
		g2::FsG2, kzg_settings::FsKZGSettings, poly::FsPoly,
	},
	utils::reverse_bit_order,
};
use scale_info::{Type, TypeInfo};

use crate::{blob::Blob, config::EMBEDDED_KZG_SETTINGS_BYTES};

// `kzg_type_with_size`宏 和 `repr_convertible`宏灵感来源于
// https://github.com/subspace/subspace/blob/main/crates/subspace-core-primitives/src/crypto/kzg.rs
// 但我们使用宏，而不是单独为每个类型都实现一遍
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
kzg_type_with_size!(KZGCommitment, FsG1, 48);
kzg_type_with_size!(KZGProof, FsG1, 48);
kzg_type_with_size!(BlsScalar, FsFr, 32);
pub trait ReprConvert<T>: Sized {
	fn slice_to_repr(value: &[Self]) -> &[T];
	fn slice_from_repr(value: &[T]) -> &[Self];
	fn vec_to_repr(value: Vec<Self>) -> Vec<T>;
	fn vec_from_repr(value: Vec<T>) -> Vec<Self>;
	fn slice_option_to_repr(value: &[Option<Self>]) -> &[Option<T>];
}

macro_rules! repr_convertible {
	($name:ident, $type:ty) => {
		impl ReprConvert<$type> for $name {
			#[inline]
			fn slice_to_repr(value: &[Self]) -> &[$type] {
				unsafe { mem::transmute(value) }
			}

			#[inline]
			fn slice_from_repr(value: &[$type]) -> &[Self] {
				unsafe { mem::transmute(value) }
			}

			#[inline]
			fn vec_to_repr(value: Vec<Self>) -> Vec<$type> {
				unsafe {
					let mut value = mem::ManuallyDrop::new(value);
					Vec::from_raw_parts(
						value.as_mut_ptr() as *mut $type,
						value.len(),
						value.capacity(),
					)
				}
			}

			#[inline]
			fn vec_from_repr(value: Vec<$type>) -> Vec<Self> {
				unsafe {
					let mut value = mem::ManuallyDrop::new(value);
					Vec::from_raw_parts(
						value.as_mut_ptr() as *mut Self,
						value.len(),
						value.capacity(),
					)
				}
			}

			#[inline]
			fn slice_option_to_repr(value: &[Option<Self>]) -> &[Option<$type>] {
				unsafe { mem::transmute(value) }
			}
		}
	};
}

// TODO: Change to automatic implementation
repr_convertible!(Blob, Vec<BlsScalar>);
repr_convertible!(KZGCommitment, FsG1);
repr_convertible!(KZGProof, FsG1);
repr_convertible!(BlsScalar, FsFr);

pub trait SafeScalar: Sized {
	const SAFE_SIZE: usize = SCALAR_SAFE_BYTES;
	fn safe_size() -> usize;
	fn try_from_bytes_safe(bytes: &[u8; SCALAR_SAFE_BYTES]) -> Result<Self, String>;
	fn to_bytes_safe(&self) -> [u8; SCALAR_SAFE_BYTES];
}

pub const SCALAR_SAFE_BYTES: usize = 31;

// BlsScalar is 32 bytes, but we only use 31 bytes for safe operations
// 32 bytes is not safe, because it can be greater than the modulus
// https://github.com/supranational/blst/blob/327d30a51c858e9c34f5b6eb3a6966b2cf6bc9cc/src/exports.c#L107
impl SafeScalar for BlsScalar {
	const SAFE_SIZE: usize = SCALAR_SAFE_BYTES;

	fn safe_size() -> usize {
		Self::SAFE_SIZE
	}

	fn try_from_bytes_safe(value: &[u8; SCALAR_SAFE_BYTES]) -> Result<Self, String> {
		let v_ptr = value.as_ptr();
		let mut full_scalar = [0u8; Self::SIZE];
		let f_ptr = full_scalar.as_mut_ptr();

		// SCALAR_SAFE_BYTES 总是小于 FULL_SCALAR_BYTES , 所以是安全的
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

		// SCALAR_SAFE_BYTES 总是小于 FULL_SCALAR_BYTES , 所以是安全的
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

#[derive(Debug, Clone, From)]
pub struct Polynomial(pub FsPoly);

impl Polynomial {
	pub fn new(size: usize) -> Result<Self, String> {
		FsPoly::new(size).map(Self)
	}

	fn is_valid(&self) -> bool {
		self.0.coeffs.len().is_power_of_two()
    }

	pub fn checked(&self) -> Result<Self, String> {
		if !self.is_valid() {
			return Err("Polynomial size must be a power of two".to_string());
		}
		Ok(self.clone())
	}

	pub fn from_coeffs(coeffs: &[FsFr]) -> Self {
		Polynomial(FsPoly { coeffs: coeffs.to_vec() })
	}

	pub fn left(&mut self) {
		let half = self.0.coeffs.len() / 2;
		self.0.coeffs.truncate(half);
	}

	pub fn to_bls_scalars(&self) -> &[BlsScalar] {
		BlsScalar::slice_from_repr(&self.0.coeffs)
	}

	pub fn to_blob(&self) -> Blob {
		Blob::from(self.to_bls_scalars().to_vec())
	}

	pub fn eval_all(&self, fs: &FsFFTSettings) -> Result<Vec<BlsScalar>, String> {
		let mut reconstructed_data = fs.fft_fr(&self.0.coeffs, false)?;
		reverse_bit_order(&mut reconstructed_data);
		Ok(BlsScalar::vec_from_repr(reconstructed_data))
	}

	pub fn eval(&self, x: &BlsScalar) -> BlsScalar {
		BlsScalar(self.0.eval(x))
	}
}

/// Number of G1 powers stored in [`EMBEDDED_KZG_SETTINGS_BYTES`]
pub const NUM_G1_POWERS: usize = 32_768;
/// Number of G2 powers stored in [`EMBEDDED_KZG_SETTINGS_BYTES`]
pub const NUM_G2_POWERS: usize = 65;

// This function clone from https://github.com/subspace/subspace/blob/main/crates/subspace-core-primitives/src/crypto/kzg.rs
// Symmetric function is present in tests
/// Function turns bytes into `FsKZGSettings`, it is up to the user to ensure that bytes make sense,
/// otherwise result can be very wrong (but will not panic).
pub fn bytes_to_kzg_settings(
	bytes: &[u8],
	num_g1_powers: usize,
	num_g2_powers: usize,
) -> Result<FsKZGSettings, String> {
	if bytes.len() != BYTES_PER_G1 * num_g1_powers + BYTES_PER_G2 * num_g2_powers {
		return Err("Invalid bytes length".to_string());
	}

	let (secret_g1_bytes, secret_g2_bytes) = bytes.split_at(BYTES_PER_G1 * num_g1_powers);
	let secret_g1 = secret_g1_bytes
		.chunks_exact(BYTES_PER_G1)
		.map(|bytes| {
			FsG1::from_bytes(
				bytes.try_into().expect("Chunked into correct number of bytes above; qed"),
			)
		})
		.collect::<Result<Vec<_>, _>>()?;
	let secret_g2 = secret_g2_bytes
		.chunks_exact(BYTES_PER_G2)
		.map(|bytes| {
			FsG2::from_bytes(
				bytes.try_into().expect("Chunked into correct number of bytes above; qed"),
			)
		})
		.collect::<Result<Vec<_>, _>>()?;

	let fft_settings = FsFFTSettings::new(
		num_g1_powers
			.checked_sub(1)
			.expect("Checked to be not empty above; qed")
			.ilog2() as usize,
	)
	.expect("Scale is within allowed bounds; qed");
	// Below is the same as `FsKZGSettings::new(&s1, &s2, num_g1_powers, &fft_settings)`, but without
	// extra checks (parameters are static anyway) and without unnecessary allocations
	Ok(FsKZGSettings { fs: fft_settings, secret_g1, secret_g2 })
}

/// Embedded KZG settings
pub fn embedded_kzg_settings() -> FsKZGSettings {
	bytes_to_kzg_settings(EMBEDDED_KZG_SETTINGS_BYTES, NUM_G1_POWERS, NUM_G2_POWERS)
		.expect("Static bytes are correct, there is a test for this; qed")
}

#[derive(Debug, Clone, AsMut)]
pub struct KZG {
	pub ks: Arc<FsKZGSettings>,
}

impl KZG {
	pub fn new(kzg_settings: FsKZGSettings) -> Self {
		Self { ks: Arc::new(kzg_settings) }
	}

	pub fn max_width(&self) -> usize {
		self.ks.fs.max_width
	}

	pub fn get_expanded_roots_of_unity_at(&self, i: usize) -> FsFr {
		self.ks.get_expanded_roots_of_unity_at(i)
	}

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

	pub fn compute_proof_multi(
		&self,
		poly: &Polynomial,
		chunk_index: usize,
		count: usize,
		chunk_size: usize,
	) -> Result<KZGProof, String> {
		// let x = self.get_expanded_roots_of_unity_at(point_indexes);
		let pos = self.get_kzg_index(count, chunk_index, chunk_size);
		let x = self.get_expanded_roots_of_unity_at(pos);
		self.ks.compute_proof_multi(&poly.0, &x, chunk_size).map(KZGProof)
	}

	pub fn check_proof_multi(
		&self,
		commitment: &KZGCommitment,
		i: usize,
		count: usize,
		values: &[FsFr],
		proof: &KZGProof,
		n: usize,
	) -> Result<bool, String> {
		let pos = self.get_kzg_index(count, i, n);
		let x = self.get_expanded_roots_of_unity_at(pos);
		self.ks.check_proof_multi(&commitment.0, &proof.0, &x, values, n)
	}

	pub fn compute_proof_with_index(
		&self,
		poly: &Polynomial,
		point_index: usize,
	) -> Result<KZGProof, String> {
		let x = self.get_expanded_roots_of_unity_at(point_index as usize);
		self.ks.compute_proof_single(&poly.0, &x).map(KZGProof)
	}

	pub fn compute_proof(&self, poly: &Polynomial, x: &FsFr) -> Result<KZGProof, String> {
		self.ks.compute_proof_single(&poly.0, &x).map(KZGProof)
	}

	pub fn commit(&self, poly: &Polynomial) -> Result<KZGCommitment, String> {
		self.ks.commit_to_poly(&poly.0).map(KZGCommitment)
	}

	pub fn verify(
		&self,
		commitment: &KZGCommitment,
		index: u32,
		value: &BlsScalar,
		proof: &KZGProof,
	) -> Result<bool, String> {
		let x = self.get_expanded_roots_of_unity_at(index as usize);

		self.ks.check_proof_single(&commitment, &proof, &x, value)
	}

	pub fn check_proof_single(
		&self,
		commitment: &KZGCommitment,
		proof: &KZGProof,
		x: &FsFr,
		value: &BlsScalar,
	) -> Result<bool, String> {
		self.ks.check_proof_single(&commitment, &proof, &x, value)
	}

	pub fn get_fs(&self) -> &FsFFTSettings {
		&self.ks.fs
	}

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

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
	vec::Vec,
};
use core::hash::{Hash, Hasher};
use core::mem;
use derive_more::{AsMut, AsRef, Deref, DerefMut, From, Into};
use kzg::{FFTSettings, Fr, KZGSettings, G1};
use parity_scale_codec::{Decode, Encode, EncodeLike, Input, MaxEncodedLen};
use rust_kzg_blst::{
	eip_4844::{
		compute_blob_kzg_proof_rust, load_trusted_setup_filename_rust,
		verify_blob_kzg_proof_batch_rust, verify_blob_kzg_proof_rust, blob_to_kzg_commitment_rust
	},
	types::{
		fft_settings::FsFFTSettings, fr::FsFr, g1::FsG1, kzg_settings::FsKZGSettings, poly::FsPoly,
	},
};
use scale_info::{Type, TypeInfo};

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

pub const BYTES_PER_BLOB: usize = BYTES_PER_FIELD_ELEMENT * FIELD_ELEMENTS_PER_BLOB;
pub const BYTES_PER_FIELD_ELEMENT: usize = 32;
pub const FIELD_ELEMENTS_PER_BLOB: usize = 4;

macro_rules! repr_convertible {
	($name:ident, $type:ty) => {
		impl $name {
			pub fn slice_to_repr(value: &[Self]) -> &[$type] {
				unsafe { mem::transmute(value) }
			}

			pub fn vec_to_repr(value: Vec<Self>) -> Vec<$type> {
				unsafe {
					let mut value = mem::ManuallyDrop::new(value);
					Vec::from_raw_parts(
						value.as_mut_ptr() as *mut $type,
						value.len(),
						value.capacity(),
					)
				}
			}
		}
	};
}

#[derive(Debug, Default, Clone, PartialEq, Eq, From, AsRef, AsMut, Deref, DerefMut)]
#[repr(transparent)]
pub struct Blob(pub Vec<FsFr>);
// TODO: Change to automatic implementation
repr_convertible!(Blob, Vec<FsFr>);
repr_convertible!(KZGCommitment, FsG1);
repr_convertible!(KZGProof, FsG1);
repr_convertible!(BlsScalar, FsFr);

impl Blob {
	#[inline]
	pub fn try_from_bytes(bytes: &[u8]) -> Result<Self, String> {
		if bytes.len() != BYTES_PER_BLOB {
			return Err(format!(
				"Invalid byte length. Expected {} got {}",
				BYTES_PER_BLOB,
				bytes.len(),
			));
		}

		Self::from_bytes(bytes).map(Self)
	}

	#[inline]
	pub fn try_from_bytes_pad(bytes: &[u8]) -> Result<Self, String> {
		if bytes.len() > BYTES_PER_BLOB {
			return Err(format!(
				"Invalid byte length. Expected maximum {} got {}",
				BYTES_PER_BLOB,
				bytes.len(),
			));
		}
		Self::from_bytes(bytes).map(|mut data| {
			if data.len() == FIELD_ELEMENTS_PER_BLOB {
				Self(data)
			} else {
				data.resize(FIELD_ELEMENTS_PER_BLOB, FsFr::zero());
				Self(data)
			}
		})
	}

	fn from_bytes(bytes: &[u8]) -> Result<Vec<FsFr>, String> {
		bytes
			.chunks(BYTES_PER_FIELD_ELEMENT)
			.map(|chunk| {
				chunk
					.try_into()
					.map_err(|_| "Chunked into incorrect number of bytes".to_string())
					.and_then(FsFr::from_bytes)
			})
			.collect()
	}

	#[inline]
	pub fn commit(&self, kzg: &KZG) -> KZGCommitment {
		KZGCommitment(blob_to_kzg_commitment_rust(&self,&kzg.settings))
	}
}

const TRUSTED_SETUP_FILENAME: &str = "eth-public-parameters.bin";

pub struct KZG {
	settings: FsKZGSettings,
}

impl KZG {
	pub fn new() -> Self {
		Self { settings: load_trusted_setup_filename_rust(TRUSTED_SETUP_FILENAME) }
	}

	pub fn compute_proof(&self, poly: &FsPoly, point_index: usize) -> Result<KZGProof, String> {
		let x = self.settings.get_expanded_roots_of_unity_at(point_index as usize);
		self.settings.compute_proof_single(poly, &x).map(KZGProof)
	}

	pub fn commit(&self, poly: &FsPoly) -> Result<KZGCommitment, String> {
		self.settings.commit_to_poly(&poly).map(KZGCommitment)
	}

	pub fn verify(
		&self,
		commitment: &KZGCommitment,
		num_values: usize,
		index: u32,
		scalar: &BlsScalar,
		proof: &KZGProof,
	) -> Result<bool, String> {
		let fft_settings = Self::new_fft_settings(num_values)?;
		let x = fft_settings.get_expanded_roots_of_unity_at(index as usize);

		self.settings.check_proof_single(&commitment, &proof, &x, scalar)
	}

	pub fn compute_blob_proof(
		&self,
		blob: &Blob,
		commitment: &KZGCommitment,
	) -> Result<KZGProof, String> {
		compute_blob_kzg_proof_rust(&blob.0, commitment, &self.settings).map(KZGProof)
	}

	pub fn verify_blob_proof(
		&self,
		blob: &Blob,
		commitment: &KZGCommitment,
		proof: &KZGProof,
	) -> Result<bool, String> {
		verify_blob_kzg_proof_rust(&blob.0, &commitment, &proof, &self.settings)
	}

	pub fn verify_blobs_proof_batch(
		&self,
		commitments: &[KZGCommitment],
		proofs: &[KZGProof],
		blobs: &[Blob],
	) -> Result<bool, String> {
		let blobs_fs_fr: &[Vec<FsFr>] = Blob::slice_to_repr(blobs);
		let commitments_fs_g1: &[FsG1] = KZGCommitment::slice_to_repr(commitments);
		let proofs_fs_g1: &[FsG1] = KZGProof::slice_to_repr(proofs);
		verify_blob_kzg_proof_batch_rust(
			blobs_fs_fr,
			commitments_fs_g1,
			proofs_fs_g1,
			&self.settings,
		)
	}

	fn new_fft_settings(num_values: usize) -> Result<FsFFTSettings, String> {
		FsFFTSettings::new(
			num_values.checked_sub(1).expect("Checked to be not empty above; qed").ilog2() as usize,
		)
	}
}

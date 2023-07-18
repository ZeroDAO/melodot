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
use derive_more::{AsMut, AsRef, Deref, DerefMut, From, Into};
use kzg::{FFTFr, FFTSettings, FK20MultiSettings, Fr, KZGSettings, DAS, FFTG1, G1, PolyRecover};
use parity_scale_codec::{Decode, Encode, EncodeLike, Input, MaxEncodedLen};
use rayon::{
	prelude::{IndexedParallelIterator, ParallelIterator},
	slice::ParallelSlice,
};
use rust_kzg_blst::{
	eip_4844::{
		blob_to_kzg_commitment_rust, compute_blob_kzg_proof_rust, load_trusted_setup_filename_rust,
		verify_blob_kzg_proof_batch_rust, verify_blob_kzg_proof_rust,
	},
	types::{
		fft_settings::FsFFTSettings, fk20_multi_settings::FsFK20MultiSettings, fr::FsFr, g1::FsG1,
		kzg_settings::FsKZGSettings, poly::FsPoly,
	},
};
use scale_info::{Type, TypeInfo};

use crate::segment::Segment;
use crate::config::SEGMENT_LENGTH;

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

pub const BYTES_PER_BLOB: usize = BYTES_PER_FIELD_ELEMENT * FIELD_ELEMENTS_PER_BLOB;
pub const BYTES_PER_FIELD_ELEMENT: usize = 32;
pub const FIELD_ELEMENTS_PER_BLOB: usize = 4;

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
		KZGCommitment(blob_to_kzg_commitment_rust(&self, &kzg.ks))
	}

	// bytes_to_blobs

	#[inline]
	pub fn segments(&self, kzg: &KZG) -> Result<Vec<Segment>, String> {
		let poly = kzg.poly(&self)?;
		let poly_len = poly.coeffs.len();
		let fk = FsFK20MultiSettings::new(&kzg.ks, 2 * poly_len, SEGMENT_LENGTH).unwrap();
		let all_proofs = fk.data_availability(&poly).unwrap();
		let segments = kzg
			.extend_blob(&self)?
			.par_chunks(SEGMENT_LENGTH)
			.enumerate()
			.map(|(i, chunk)| {
				let position = Position::default();
				let content = chunk;
				let proof = all_proofs[i];
				Segment::new(position, BlsScalar::slice_from_repr(content) , KZGProof(proof))
			})
			.collect::<Vec<_>>();
		Ok(segments)
	}
}

#[derive(Debug, Clone, From)]
pub struct Polynomial(FsPoly);

impl Polynomial {
    pub fn normalize(&mut self) {
        let trailing_zeroes = self
            .0
            .coeffs
            .iter()
            .rev()
            .take_while(|coeff| coeff.is_zero())
            .count();
        self.0
            .coeffs
            .truncate((self.0.coeffs.len() - trailing_zeroes).max(1));
    }

	pub fn to_bls_scalars(&self) -> &[BlsScalar] {
		BlsScalar::slice_from_repr(&self.0.coeffs)
	}
}

const TRUSTED_SETUP_FILENAME: &str = "eth-public-parameters.bin";

pub struct KZG {
	ks: Arc<FsKZGSettings>,
	fs: Arc<FsFFTSettings>,
}

impl KZG {
	pub fn new() -> Self {
		Self {
			ks: Arc::new(load_trusted_setup_filename_rust(TRUSTED_SETUP_FILENAME)),
			fs: Arc::new(FsFFTSettings::default()),
		}
	}

	pub fn max_width(&self) -> usize {
		self.fs.max_width
	}

	pub fn get_expanded_roots_of_unity_at(&self, i: usize) -> FsFr {
		self.fs.get_expanded_roots_of_unity_at(i)
	}

	pub fn get_kzg_index(&self, chunk_index: usize) -> usize {
		let domain_stride = self.max_width() / (2 * FIELD_ELEMENTS_PER_BLOB);
		let chunk_count = FIELD_ELEMENTS_PER_BLOB / SEGMENT_LENGTH;
		let domain_pos = Self::reverse_bits_limited(chunk_count, chunk_index);
		domain_pos * domain_stride
	}

	pub fn compute_proof_multi(
		&self,
		poly: &Polynomial,
		point_indexes: usize,
		n: usize,
	) -> Result<KZGProof, String> {
		let x = self
			.fs
			.get_expanded_roots_of_unity_at(point_indexes);
		self.ks.compute_proof_multi(&poly.0, &x, n).map(KZGProof)
	}

	pub fn check_proof_multi(
		&self,
		commitment: &KZGCommitment,
		i: usize,
		values: &[FsFr],
		proof: &KZGProof,
	) -> Result<bool, String> {
		let x = self.fs.get_expanded_roots_of_unity_at(i);
		self.ks
			.check_proof_multi(&commitment.0,&proof.0, &x, values ,FIELD_ELEMENTS_PER_BLOB)
	}

	pub fn compute_proof(&self, poly: &FsPoly, point_index: usize) -> Result<KZGProof, String> {
		let x = self.ks.get_expanded_roots_of_unity_at(point_index as usize);
		self.ks.compute_proof_single(poly, &x).map(KZGProof)
	}

	pub fn commit(&self, poly: &FsPoly) -> Result<KZGCommitment, String> {
		self.ks.commit_to_poly(&poly).map(KZGCommitment)
	}

	pub fn verify(
		&self,
		commitment: &KZGCommitment,
		index: u32,
		scalar: &BlsScalar,
		proof: &KZGProof,
	) -> Result<bool, String> {
		let x = self.fs.get_expanded_roots_of_unity_at(index as usize);

		self.ks.check_proof_single(&commitment, &proof, &x, scalar)
	}

	pub fn compute_blob_proof(
		&self,
		blob: &Blob,
		commitment: &KZGCommitment,
	) -> Result<KZGProof, String> {
		compute_blob_kzg_proof_rust(&blob.0, commitment, &self.ks).map(KZGProof)
	}

	pub fn verify_blob_proof(
		&self,
		blob: &Blob,
		commitment: &KZGCommitment,
		proof: &KZGProof,
	) -> Result<bool, String> {
		verify_blob_kzg_proof_rust(&blob.0, &commitment, &proof, &self.ks)
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
		verify_blob_kzg_proof_batch_rust(blobs_fs_fr, commitments_fs_g1, proofs_fs_g1, &self.ks)
	}

	pub fn new_fft_settings(&mut self, num_values: usize) -> Result<(), String> {
		self.fs = Arc::new(FsFFTSettings::new(
			num_values.checked_sub(1).expect("Checked to be not empty above; qed").ilog2() as usize,
		)?);
		Ok(())
	}

	pub fn poly(&self, blob: &Blob) -> Result<FsPoly, String> {
		// self.new_fft_settings(data.len())?;
		let poly = FsPoly { coeffs: self.fs.fft_fr(&blob.0, true)? };
		Ok(poly)
	}

	pub fn extend_blob(&self, blob: &Blob) -> Result<Vec<FsFr>, String> {
		self.fs.das_fft_extension(&blob.0)
	}

	pub fn extend_fs_g1<T: ReprConvert<FsG1>>(&self, source: &[T]) -> Result<Vec<T>, String> {
		let mut coeffs = self.fs.fft_g1(T::slice_to_repr(source), true)?;

		coeffs.resize(coeffs.len() * 2, FsG1::identity());

		self.fs.fft_g1(&coeffs, false).map(T::vec_from_repr)
	}

    pub fn recover_poly(&self, shards: &[Option<BlsScalar>]) -> Result<Polynomial, String> {
		let poly = FsPoly::recover_poly_from_samples(
			BlsScalar::slice_option_to_repr(shards),
			&self.fs,
		)?;
		Ok(Polynomial::from(poly))
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
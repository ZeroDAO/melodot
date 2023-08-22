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

use crate::{
	kzg::{BlsScalar, KZGCommitment, KZGProof, ReprConvert, SafeScalar, KZG, SCALAR_SAFE_BYTES},
	polynomial::Polynomial,
};
use alloc::{
	string::{String, ToString},
	vec::Vec,
};
use derive_more::{AsMut, AsRef, Deref, DerefMut, From};
use kzg::eip_4844::{bytes_of_uint64, hash, CHALLENGE_INPUT_SIZE, FIAT_SHAMIR_PROTOCOL_DOMAIN};
use kzg::{Fr, G1};

use rust_kzg_blst::{
	eip_4844::hash_to_bls_field,
	eip_4844::verify_kzg_proof_batch,
	types::{fr::FsFr, g1::FsG1, poly::FsPoly},
};

use crate::config::BYTES_PER_FIELD_ELEMENT;

/// A blob is a vector of field elements. It is the basic unit of data that is
/// stored in the data availability layer.
#[derive(Debug, Default, Clone, PartialEq, Eq, From, AsRef, AsMut, Deref, DerefMut)]
#[repr(transparent)]
pub struct Blob(pub Vec<BlsScalar>);

impl Blob {
	/// Creates a new blob from a vector of field elements.
	///
	/// It takes a bytes_per_blob parameter to validate the length of the bytes input,
	/// which should be a constant
	/// value for convenience. We pass it as a parameter to make it more flexible, and
	///  the compiler should optimize it.
	#[inline]
	pub fn try_from_bytes(bytes: &[u8], bytes_per_blob: usize) -> Result<Self, String> {
		if bytes.len() != bytes_per_blob {
			return Err(format!(
				"Invalid byte length. Expected {} got {}",
				bytes_per_blob,
				bytes.len(),
			));
		}

		Self::from_bytes(bytes).map(Self)
	}

	/// Attempts to create a new `Self` instance from a byte slice `bytes` with padding.
	///
	/// # Arguments
	///
	/// * `bytes` - A byte slice to create the instance from.
	/// * `bytes_per_blob` - The expected byte length.
	///
	/// # Errors
	///
	/// Returns an error if the length of `bytes` is greater than `bytes_per_blob`.
	///
	/// # Returns
	///
	/// Returns a `Result` containing the new `Self` instance or an error message.
	#[inline]
	pub fn try_from_bytes_pad(bytes: &[u8], bytes_per_blob: usize) -> Result<Self, String> {
		if bytes.len() > bytes_per_blob {
			return Err(format!(
				"Invalid byte length. Expected maximum {} got {}",
				bytes_per_blob,
				bytes.len(),
			));
		}
		let mut bytes_vec = bytes.to_vec();
		bytes_vec.resize(bytes_per_blob, 0);

		Self::from_bytes(&bytes_vec).map(Self)
	}

	fn from_bytes(bytes: &[u8]) -> Result<Vec<BlsScalar>, String> {
		bytes
			.chunks(SCALAR_SAFE_BYTES)
			.map(|chunk| {
				chunk
					.try_into()
					.map_err(|_| "Chunked into incorrect number of bytes".to_string())
					.and_then(<BlsScalar as crate::kzg::SafeScalar>::try_from_bytes_safe)
			})
			.collect()
	}

	/// Converts the `Self` instance to a byte vector.
	///
	/// # Returns
	///
	/// Returns a `Vec` of bytes representing the `Self` instance.
	#[inline]
	pub fn to_bytes(&self) -> Vec<u8> {
		self.0.iter().flat_map(|scalar| scalar.to_bytes_safe()).collect()
	}

	/// Converts the `Self` instance to a byte vector of length `len`.
	/// If `len` is less than the length of the `Self` instance, it will be truncated.
	/// If `len` is greater than the length of the `Self` instance, it will be padded with zeros.
	#[inline]
	pub fn to_bytes_by_len(&self, len: usize) -> Vec<u8> {
		let mut bytes = self.to_bytes();
		if len < bytes.len() {
			bytes.resize(len, 0);
			bytes
		} else {
			bytes
		}
	}

	/// Commits the `Self` instance using the provided `KZG` scheme.
	///
	/// # Arguments
	///
	/// * `kzg` - A reference to a `KZG` scheme.
	///
	/// # Returns
	///
	/// Returns a `Result` containing the `KZGCommitment` or an error message.
	#[inline]
	pub fn commit(&self, kzg: &KZG) -> Result<KZGCommitment, String> {
		let poly = self.to_poly();
		kzg.commit(&poly)
	}

	/// Computes a KZG proof for the `Self` instance using the provided `KZG` scheme.
	///
	/// # Arguments
	/// * `commitment` - A reference to a `KZGCommitment`.
	/// * `kzg` - A reference to a `KZG` scheme.
	/// * `field_elements_per_blob` - The number of field elements per blob.
	///
	/// # Returns
	/// Returns a `Result` containing the `KZGProof` or an error message.
	#[inline]
	pub fn kzg_proof(
		&self,
		commitment: &KZGCommitment,
		kzg: &KZG,
		field_elements_per_blob: usize,
	) -> Result<KZGProof, String> {
		check_field_elements_per_blob(field_elements_per_blob)?;
		let bytes_per_blob: usize = BYTES_PER_FIELD_ELEMENT * field_elements_per_blob;

		let x = compute_challenge(
			&self.to_fs_fr_vec(),
			commitment,
			bytes_per_blob,
			field_elements_per_blob,
		);
		let poly = self.to_poly();
		kzg.compute_proof(&poly, &x)
	}

	/// Computes a KZG commitment and proof for the `Self` instance using the provided `KZG` scheme.
	///
	/// # Arguments
	/// * `kzg` - A reference to a `KZG` scheme.
	/// * `field_elements_per_blob` - The number of field elements per blob.
	///
	/// # Returns
	/// Returns a `Result` containing the `KZGCommitment` and `KZGProof` or an error message.
	#[inline]
	pub fn commit_and_proof(
		&self,
		kzg: &KZG,
		field_elements_per_blob: usize,
	) -> Result<(KZGCommitment, KZGProof), String> {
		check_field_elements_per_blob(field_elements_per_blob)?;
		let bytes_per_blob: usize = BYTES_PER_FIELD_ELEMENT * field_elements_per_blob;

		let poly = self.to_poly();
		let commitment = kzg.commit(&poly)?;

		let x = compute_challenge(
			&self.to_fs_fr_vec(),
			&commitment,
			bytes_per_blob,
			field_elements_per_blob,
		);
		let proof = kzg.compute_proof(&poly, &x)?;

		Ok((commitment, proof))
	}

	/// Verifies a KZG proof for the `Self` instance using the provided `KZG` scheme.
	///
	/// # Arguments
	/// * `commitment` - A reference to a `KZGCommitment`.
	/// * `proof` - A reference to a `KZGProof`.
	/// * `kzg` - A reference to a `KZG` scheme.
	/// * `field_elements_per_blob` - The number of field elements per blob.
	///
	/// # Returns
	/// Returns a `Result` containing a boolean indicating whether the proof is valid or an error message.
	#[inline]
	pub fn verify(
		&self,
		kzg: &KZG,
		commitment: &KZGCommitment,
		proof: &KZGProof,
		field_elements_per_blob: usize,
	) -> Result<bool, String> {
		check_field_elements_per_blob(field_elements_per_blob)?;
		let bytes_per_blob: usize = BYTES_PER_FIELD_ELEMENT * field_elements_per_blob;

		let x = compute_challenge(
			&self.to_fs_fr_vec(),
			commitment,
			bytes_per_blob,
			field_elements_per_blob,
		);
		let poly = self.to_poly();
		let y = poly.eval(&BlsScalar(x));
		kzg.check_proof_single(commitment, proof, &x, &y)
	}

	/// Verifies a batch of KZG proofs for the `Self` instance using the provided `KZG` scheme.
	///
	/// # Arguments
	/// * `blobs` - A slice of `Blob`s.
	/// * `commitments` - A slice of `KZGCommitment`s.
	/// * `proofs` - A slice of `KZGProof`s.
	/// * `kzg` - A reference to a `KZG` scheme.
	/// * `field_elements_per_blob` - The number of field elements per blob.
	///
	/// # Returns
	/// Returns a `Result` containing a boolean indicating whether the proofs are valid or an error message.
	pub fn verify_batch(
		blobs: &[Blob],
		commitments: &[KZGCommitment],
		proofs: &[KZGProof],
		kzg: &KZG,
		field_elements_per_blob: usize,
	) -> Result<bool, String> {
		if commitments.iter().any(|commitment| !commitment.0.is_valid()) {
			return Err("Invalid commitment".to_string());
		}

		if proofs.iter().any(|proof| !proof.0.is_valid()) {
			return Err("Invalid proof".to_string());
		}

		// Check that the lengths of commitment, blobs, and proof are the same.
		if blobs.len() != commitments.len() || blobs.len() != proofs.len() {
			return Err(format!(
				"Invalid input length. Expected {} got commitments: {} and proofs: {}",
				blobs.len(),
				commitments.len(),
				proofs.len()
			));
		}

		check_field_elements_per_blob(field_elements_per_blob)?;
		let bytes_per_blob: usize = BYTES_PER_FIELD_ELEMENT * field_elements_per_blob;

		let (zs, ys) = compute_challenges_and_evaluate_polynomial(
			blobs,
			commitments,
			bytes_per_blob,
			field_elements_per_blob,
		);

		Ok(verify_kzg_proof_batch(
			KZGCommitment::slice_to_repr(commitments),
			&zs,
			&ys,
			KZGProof::slice_to_repr(proofs),
			&kzg.ks,
		))
	}

	/// Converts the `Self` instance to a `Polynomial`.
	///
	/// # Returns
	///
	/// Returns a `Polynomial` instance.
	#[inline]
	pub fn to_poly(&self) -> Polynomial {
		Polynomial(FsPoly { coeffs: self.to_fs_fr_vec() })
	}

	/// Converts the `Self` instance to a `&[FsFr]`.
	///
	/// It will convert directly instead of copying
	#[inline]
	pub fn to_fs_fr_slice(&self) -> &[FsFr] {
		BlsScalar::slice_to_repr(&self.0)
	}

	/// Converts the `Self` instance to a `Vec<FsFr>`.
	#[inline]
	pub fn to_fs_fr_vec(&self) -> Vec<FsFr> {
		BlsScalar::vec_to_repr(self.0.clone())
	}

	#[inline]
	pub fn blob_count(bytes_len: usize, bytes_per_blob: usize) -> usize {
		(bytes_len + bytes_per_blob - 1) / bytes_per_blob
	}
}

// field_elements_per_blob should be a power of 2
fn check_field_elements_per_blob(field_elements_per_blob: usize) -> Result<(), String> {
	if !field_elements_per_blob.is_power_of_two() {
		return Err("field_elements_per_blob must be powers of two".to_string());
	}
	Ok(())
}

// Calculate the challenge and return the evaluated value at the challenge value
fn compute_challenges_and_evaluate_polynomial(
	blobs: &[Blob],
	commitments: &[KZGCommitment],
	bytes_per_blob: usize,
	field_elements_per_blob: usize,
) -> (Vec<FsFr>, Vec<FsFr>) {
	blobs.iter().zip(commitments.iter()).fold(
		(Vec::with_capacity(blobs.len()), Vec::with_capacity(blobs.len())),
		|(mut zs, mut ys), (blob, commitment)| {
			let poly = blob.to_poly();
			let fs_fr_vec = blob.to_fs_fr_vec();
			let z =
				compute_challenge(&fs_fr_vec, commitment, bytes_per_blob, field_elements_per_blob);
			let y = poly.eval(&BlsScalar(z));

			zs.push(z);
			ys.push(y.0);

			(zs, ys)
		},
	)
}

// This is a copy from kzg-rust https://github.com/sifraitech/rust-kzg/blob/main/blst/src/eip_4844.rs#L337
// Used to calculate the challenge value for the Blob, where we pass in the constant field_elements_per_blob
// as a parameter for ease of use by the application layer
fn compute_challenge(
	blob: &[FsFr],
	commitment: &FsG1,
	bytes_per_blob: usize,
	field_elements_per_blob: usize,
) -> FsFr {
	let mut bytes: Vec<u8> = vec![0; CHALLENGE_INPUT_SIZE];

	// Copy domain separator
	bytes[..16].copy_from_slice(&FIAT_SHAMIR_PROTOCOL_DOMAIN);
	bytes_of_uint64(&mut bytes[16..24], field_elements_per_blob as u64);
	// Set all other bytes of this 16-byte (little-endian) field to zero
	bytes_of_uint64(&mut bytes[24..32], 0);

	// Copy blob
	for i in 0..blob.len() {
		let v = blob[i].to_bytes();
		bytes[(32 + i * BYTES_PER_FIELD_ELEMENT)..(32 + (i + 1) * BYTES_PER_FIELD_ELEMENT)]
			.copy_from_slice(&v);
	}

	// Copy commitment
	let v = commitment.to_bytes();
	for i in 0..v.len() {
		bytes[32 + bytes_per_blob + i] = v[i];
	}

	// Now let's create the challenge!
	let eval_challenge = hash(&bytes);
	hash_to_bls_field(&eval_challenge)
}

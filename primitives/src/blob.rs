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

use crate::kzg::{
	BlsScalar, KZGCommitment, KZGProof, Polynomial, ReprConvert, SafeScalar, KZG, SCALAR_SAFE_BYTES,
};
use alloc::{
	string::{String, ToString},
	vec::Vec,
};
use derive_more::{AsMut, AsRef, Deref, DerefMut, From};
use kzg::eip_4844::{
	bytes_of_uint64, hash, BYTES_PER_FIELD_ELEMENT, CHALLENGE_INPUT_SIZE,
	FIAT_SHAMIR_PROTOCOL_DOMAIN,
};
use kzg::{Fr, G1};

use rust_kzg_blst::{
	eip_4844::hash_to_bls_field,
	eip_4844::verify_kzg_proof_batch,
	types::{fr::FsFr, g1::FsG1, poly::FsPoly},
};

#[derive(Debug, Default, Clone, PartialEq, Eq, From, AsRef, AsMut, Deref, DerefMut)]
#[repr(transparent)]
pub struct Blob(pub Vec<BlsScalar>);

impl Blob {
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

	#[inline]
	pub fn to_bytes(&self) -> Vec<u8> {
		self.0.iter().flat_map(|scalar| scalar.to_bytes_safe()).collect()
	}

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

	#[inline]
	pub fn commit(&self, kzg: &KZG) -> Result<KZGCommitment, String> {
		let poly = self.to_poly();
		kzg.commit(&poly)
	}

	#[inline]
	pub fn kzg_proof(
		&self,
		commitment: &KZGCommitment,
		kzg: &KZG,
		bytes_per_blob: usize,
		field_elements_per_blob: usize,
	) -> Result<KZGProof, String> {
		let x = compute_challenge(
			&self.to_fs_fr_vec(),
			commitment,
			bytes_per_blob,
			field_elements_per_blob,
		);
		let poly = self.to_poly();
		kzg.compute_proof(&poly, &x)
	}

	#[inline]
	pub fn commit_and_proof(
		&self,
		kzg: &KZG,
		bytes_per_blob: usize,
		field_elements_per_blob: usize,
	) -> Result<(KZGCommitment, KZGProof), String> {
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

	#[inline]
	pub fn verify(
		&self,
		kzg: &KZG,
		commitment: &KZGCommitment,
		proof: &KZGProof,
		bytes_per_blob: usize,
		field_elements_per_blob: usize,
	) -> Result<bool, String> {
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

	pub fn verify_batch(
		blobs: &[Blob],
		commitments: &[KZGCommitment],
		proofs: &[KZGProof],
		kzg: &KZG,
		bytes_per_blob: usize,
		field_elements_per_blob: usize,
	) -> Result<bool, String> {
		// validate_batched_input(commitments_g1, proofs_g1)?;
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

	#[inline]
	pub fn to_poly(&self) -> Polynomial {
		Polynomial(FsPoly { coeffs: self.to_fs_fr_vec() })
	}

	#[inline]
	pub fn to_fs_fr_slice(&self) -> &[FsFr] {
		BlsScalar::slice_to_repr(&self.0)
	}

	#[inline]
	pub fn to_fs_fr_vec(&self) -> Vec<FsFr> {
		BlsScalar::vec_to_repr(self.0.clone())
	}
}

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

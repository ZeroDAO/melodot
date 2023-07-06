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

use super::kzg_data::{Blob, BlsScalar, KZGCommitment, KZGProof};

use alloc::string::String;
use alloc::vec::Vec;
use kzg::{FFTSettings, KZGSettings,};
use rust_kzg_blst::{
	eip_4844::{
		compute_blob_kzg_proof_rust, load_trusted_setup_filename_rust,
		verify_blob_kzg_proof_batch_rust, verify_blob_kzg_proof_rust,
	},
	types::{
		fft_settings::FsFFTSettings, fr::FsFr, g1::FsG1, kzg_settings::FsKZGSettings, poly::FsPoly,
	},
};

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
		let blobs_fs_fr: Vec<Vec<FsFr>> = blobs.iter().map(|blob| blob.0.clone()).collect();
		let commitments_fs_g1: Vec<FsG1> =
			commitments.iter().map(|commitment| commitment.0).collect();
		let proofs_fs_g1: Vec<FsG1> = proofs.iter().map(|proof| proof.0).collect();
		verify_blob_kzg_proof_batch_rust(
			&blobs_fs_fr,
			&commitments_fs_g1,
			&proofs_fs_g1,
			&self.settings,
		)
	}

	fn new_fft_settings(num_values: usize) -> Result<FsFFTSettings, String> {
		FsFFTSettings::new(
			num_values.checked_sub(1).expect("Checked to be not empty above; qed").ilog2() as usize,
		)
	}
}
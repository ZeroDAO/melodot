// Copyright 2023 ZeroDAO

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at

//     http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::melodot::runtime_types::melo_das_primitives::crypto::{KZGCommitment, KZGProof};
use crate::Client;
use melo_core_primitives::SidercarMetadata;
use melo_das_primitives::crypto::{KZGCommitment as KZGCommitmentT, KZGProof as KZGProofT};

pub use primitive_types::H256;

pub fn sidercar_metadata_runtime(
	bytes_len: u32,
) -> (Vec<KZGCommitment>, Vec<KZGProof>, H256, Vec<u8>) {
	let (commits, proofs, blobs_hash, bytes) = sidercar_metadata(bytes_len);
	(commitments_to_runtime(commits), proofs_to_runtime(proofs), blobs_hash, bytes)
}

pub fn sidercar_metadata(bytes_len: u32) -> (Vec<KZGCommitmentT>, Vec<KZGProofT>, H256, Vec<u8>) {
	let bytes = (0..bytes_len).map(|_| rand::random::<u8>()).collect::<Vec<u8>>();
	let metadata: SidercarMetadata = SidercarMetadata::try_from_app_data(&bytes).unwrap();
	(metadata.commitments, metadata.proofs, metadata.blobs_hash, bytes)
}

pub fn commitments_to_runtime(commitments: Vec<KZGCommitmentT>) -> Vec<KZGCommitment> {
	commitments
		.iter()
		.map(|c| KZGCommitment { inner: c.to_bytes() })
		.collect::<Vec<_>>()
}

pub fn proofs_to_runtime(proofs: Vec<KZGProofT>) -> Vec<KZGProof> {
	proofs.iter().map(|c| KZGProof { inner: c.to_bytes() }).collect::<Vec<_>>()
}

pub async fn wait_for_block(client: &Client) -> Result<(), Box<dyn std::error::Error>> {
    let mut sub = client.api.rpc().subscribe_all_block_headers().await?;
    sub.next().await;
	sub.next().await;
	
	Ok(())
}

pub mod info_msg {
	pub const START_EXAMPLE: &str = "🌟 Start";
	pub const ERROR: &str = "❌ Error";
	pub const SUCCESS: &str = "✅ Success";
	pub const ALL_SUCCESS: &str = "💯 All success";
	pub const HOURGLASS: &str = "⏳";
}

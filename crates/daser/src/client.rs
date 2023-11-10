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
use crate::{
	anyhow, Arc, Context, DasKv, DasNetworkOperations, KZGCommitment, Ok, Reliability,
	ReliabilityId, Result, SAMPLES_PER_BLOCK,
};

use codec::{Decode, Encode};
use futures::lock::Mutex;
use log::{info, debug};
use melo_core_primitives::{
	reliability::{ReliabilitySample, ReliabilityType},
	traits::HeaderWithCommitment,
	AppLookup,
};
use melo_erasure_coding::erasure_coding::extend_fs_g1;
use std::marker::PhantomData;

/// The key used to store the last block number sampled.
const LAST_AT_KEY: &[u8] = b"sampled_at_last_block";

/// The client used to sample the network.
pub struct SamplingClient<Header, DB, DaserNetwork>
where
	DaserNetwork: DasNetworkOperations + Sync,
{
	/// The network used to fetch samples.
	pub network: DaserNetwork,
	database: Arc<Mutex<DB>>,
	_phantom: PhantomData<Header>,
}

/// A trait for sampling an application and block.
#[async_trait::async_trait]
pub trait Sampling {
	/// Samples the application.
	///
	/// # Arguments
	///
	/// * `app_id` - The ID of the application to sample.
	/// * `nonce` - A nonce value.
	/// * `commitments` - An array of KZG commitments.
	///
	/// # Returns
	///
	/// Returns `Ok(())` if the sampling is successful, otherwise returns an error.
	async fn sample_application(
		&self,
		app_id: u32,
		nonce: u32,
		commitments: &[KZGCommitment],
	) -> Result<()>;

	/// Samples the block.
	///
	/// # Arguments
	///
	/// * `header` - A reference to the block header.
	///
	/// # Returns
	///
	/// Returns `Ok(())` if the sampling is successful, otherwise returns an error.
	async fn sample_block<Header>(&self, header: &Header) -> Result<()>
	where
		Header: HeaderWithCommitment + Sync;

	/// Returns the last block number sampled.
	///
	/// # Returns
	///
	/// Returns the last block number sampled.
	async fn last_at(&self) -> u32;
}

impl<Header, DB: DasKv, DaserNetwork: DasNetworkOperations> SamplingClient<Header, DB, DaserNetwork>
where
	DaserNetwork: DasNetworkOperations + Sync,
{
	/// Creates a new [`SamplingClient`] instance.
	pub fn new(network: DaserNetwork, database: Arc<Mutex<DB>>) -> Self {
		SamplingClient { network, database, _phantom: PhantomData }
	}

	/// Actually samples the network.
	async fn sample(
		&self,
		confidence_id: &ReliabilityId,
		confidence: &mut Reliability,
		commitments: &[KZGCommitment],
	) -> Result<()> {
		for (sample, commitment) in confidence.samples.iter_mut().zip(commitments.iter()) {
			if self.network.fetch_sample(sample, commitment).await.is_some() {
				sample.set_success();
			} else {
				debug!("Sampled failed: {:?}", sample.id);
			}
		}

		let mut db_guard = self.database.lock().await;

		confidence.save(confidence_id, &mut *db_guard);

		Ok(())
	}

	/// Sets the last block number sampled.
	async fn set_last_at<Number>(&self, last: Number)
	where
		Number: Encode + Decode + PartialOrd + Send,
	{
		let mut db_guard = self.database.lock().await;
		let should_update = match db_guard.get(LAST_AT_KEY) {
			Some(bytes) =>
				if let core::result::Result::Ok(current_last) = Number::decode(&mut &bytes[..]) {
					last > current_last
				} else {
					true
				},
			None => true,
		};

		if should_update {
			let encoded = last.encode();
			db_guard.set(LAST_AT_KEY, &encoded);
		}
	}
}

#[async_trait::async_trait]
impl<H: HeaderWithCommitment + Sync, DB: DasKv + Send, D: DasNetworkOperations + Sync> Sampling
	for SamplingClient<H, DB, D>
{
	/// Get the last block number sampled.
	async fn last_at(&self) -> u32 {
		let mut db_guard = self.database.lock().await;
		db_guard
			.get(LAST_AT_KEY)
			.and_then(|bytes| Decode::decode(&mut &bytes[..]).ok())
			.unwrap_or(0u32)
	}

	/// Samples the application.
	async fn sample_application(
		&self,
		app_id: u32,
		nonce: u32,
		commitments: &[KZGCommitment],
	) -> Result<()> {
		let id = ReliabilityId::app_confidence(app_id, nonce);
		let mut confidence = Reliability::new(ReliabilityType::App, commitments);
		let blob_count = commitments.len();
		let n = blob_count;
		let app_lookups = vec![AppLookup { app_id, nonce, count: blob_count as u16 }];
		let sample_commitments =
			confidence.set_sample(n, &app_lookups, None).map_err(|e| anyhow!(e))?;
		self.sample(&id, &mut confidence, &sample_commitments).await
	}

	/// Samples the block.
	async fn sample_block<Header>(&self, header: &Header) -> Result<()>
	where
		Header: HeaderWithCommitment + Sync,
	{
		let block_hash = header.hash().encode();
		let id = ReliabilityId::block_confidence(&block_hash);
		let commitments = header.commitments().context("Commitments not found in the header")?;

		if !commitments.is_empty() {
			info!("🌈 Sampling block {}", header.number());

			let extended_commits =
				extend_fs_g1(self.network.kzg().get_fs(), &commitments).map_err(|e| anyhow!(e))?;
			let mut confidence = Reliability::new(ReliabilityType::Block, &extended_commits);

			let app_lookups = header.extension().app_lookup.clone();

			let sample_commitments = confidence
				.set_sample(SAMPLES_PER_BLOCK, &app_lookups, Some(&block_hash))
				.map_err(|e| anyhow!(e))?;

			self.sample(&id, &mut confidence, &sample_commitments).await?;
		}

		let at = header.number();
		self.set_last_at::<<Header as HeaderWithCommitment>::Number>(*at).await;
		Ok(())
	}
}

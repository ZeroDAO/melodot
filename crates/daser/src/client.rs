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
	ReliabilityId, Result, SAMPLES_PER_BLOB,
};

use codec::{Decode, Encode};
use futures::lock::Mutex;
use melo_core_primitives::{
	reliability::{ReliabilitySample, ReliabilityType},
	traits::HeaderWithCommitment,
};
// use sp_api::HeaderT;
use std::marker::PhantomData;

const LAST_AT_KEY: &[u8] = b"sampled_at_last_block";
pub struct SamplingClient<Header, DB, DaserNetwork>
where
	DaserNetwork: DasNetworkOperations + Sync,
{
	pub network: DaserNetwork,
	database: Arc<Mutex<DB>>,
	_phantom: PhantomData<Header>,
}

#[async_trait::async_trait]
pub trait Sampling {
	async fn sample_application(
		&self,
		app_id: u32,
		nonce: u32,
		commitments: &Vec<KZGCommitment>,
	) -> Result<()>;

	async fn sample_block<Header>(&self, header: &Header) -> Result<()>
	where
		Header: HeaderWithCommitment + Sync;

	async fn last_at(&self) -> u32;
}

impl<Header, DB: DasKv, DaserNetwork: DasNetworkOperations> SamplingClient<Header, DB, DaserNetwork>
where
	DaserNetwork: DasNetworkOperations + Sync,
{
	pub fn new(network: DaserNetwork, database: Arc<Mutex<DB>>) -> Self {
		SamplingClient { network, database, _phantom: PhantomData }
	}

	async fn sample(
		&self,
		confidence_id: &ReliabilityId,
		confidence: &mut Reliability,
		app: &[(u32, u32)],
		commitments: &[KZGCommitment],
	) -> Result<()> {
		if confidence.samples.len() != app.len() || commitments.len() != app.len() {
			return Err(anyhow!("nvalid sample length"))
		}

		for ((sample, (app_id, nonce)), commitment) in
			confidence.samples.iter_mut().zip(app.iter()).zip(commitments.iter())
		{
			if self.network.fetch_sample(*app_id, *nonce, sample, commitment).await.is_some() {
				sample.set_success();
			}
		}

		let mut db_guard = self.database.lock().await;

		confidence.save(confidence_id, &mut *db_guard);

		Ok(())
	}

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
			let _ = db_guard.set(LAST_AT_KEY, &encoded);
		}
	}
}

#[async_trait::async_trait]
impl<H: HeaderWithCommitment + Sync, DB: DasKv + Send, D: DasNetworkOperations + Sync> Sampling
	for SamplingClient<H, DB, D>
{
	async fn last_at(&self) -> u32 {
		let mut db_guard = self.database.lock().await;
		db_guard
			.get(LAST_AT_KEY)
			.map(|bytes| Decode::decode(&mut &bytes[..]).ok())
			.flatten()
			.unwrap_or_else(|| 0u32)
	}

	async fn sample_application(
		&self,
		app_id: u32,
		nonce: u32,
		commitments: &Vec<KZGCommitment>,
	) -> Result<()> {
		let id = ReliabilityId::app_confidence(app_id, nonce);

		let mut confidence = Reliability::new(ReliabilityType::App, &commitments);

		let blob_count = commitments.len();

		// 平均每个blob抽样1次
		let n = blob_count;

		confidence.set_sample(n);

		let apps = vec![(app_id, nonce); blob_count];

		self.sample(&id, &mut confidence, &apps, &commitments).await
	}

	async fn sample_block<Header>(&self, header: &Header) -> Result<()>
	where
		Header: HeaderWithCommitment + Sync,
	{
		let id = ReliabilityId::block_confidence(&header.hash().encode());
		let commitments = header.commitments().context("Commitments not found in the header")?;

		if commitments.len() > 0 {
			let mut confidence = Reliability::new(ReliabilityType::Block, &commitments);

			confidence.set_sample(SAMPLES_PER_BLOB);

			let apps: Result<Vec<(u32, u32)>> = confidence
				.samples
				.iter()
				.map(|sample| {
					header
						.extension()
						.get_lookup(sample.position.y)
						.context("AppLookup not found for given sample position")
						.map(|app_lookup| (app_lookup.app_id, app_lookup.nonce))
				})
				.collect();

			let apps = apps?;

			self.sample(&id, &mut confidence, &apps, &commitments).await?;
		}

		let at = header.number();
		self.set_last_at::<<Header as HeaderWithCommitment>::Number>(*at).await;
		Ok(())
	}
}

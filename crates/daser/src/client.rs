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
use crate::{Arc, Confidence, ConfidenceId, KZGCommitment, NetworkDas, SAMPLES_PER_BLOB};

use codec::{Decode, Encode};
use futures::lock::Mutex;
use melo_core_primitives::{confidence::ConfidenceSample, traits::ExtendedHeader};
use sp_api::HeaderT;
use std::marker::PhantomData;

const LAST_AT_KEY: &[u8] = b"sampled_at_last_block";
pub struct SamplingClient<Header, DB> {
	pub network: NetworkDas,
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
	) -> Result<(), Box<dyn std::error::Error>>;

	async fn sample_block<Header>(&self, header: &Header) -> Result<(), Box<dyn std::error::Error>>
	where
		Header: ExtendedHeader + HeaderT;

	async fn last_at(&self) -> u32;
}

impl<Header, DB: melo_das_db::traits::DasKv> SamplingClient<Header, DB> {
	pub fn new(network: NetworkDas, database: DB) -> Self {
		SamplingClient { network, database: Arc::new(Mutex::new(database)), _phantom: PhantomData }
	}

	async fn sample(
		&self,
		confidence_id: &ConfidenceId,
		confidence: &mut Confidence,
		app: &[(u32, u32)],
		commitments: &[KZGCommitment],
	) -> Result<(), Box<dyn std::error::Error>> {
		if confidence.samples.len() != app.len() || commitments.len() != app.len() {
			return Err(Box::new(std::io::Error::new(
				std::io::ErrorKind::InvalidData,
				"Invalid sample length",
			)))
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
				if let Ok(current_last) = Number::decode(&mut &bytes[..]) {
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
impl<H: HeaderT, DB: melo_das_db::traits::DasKv + Send> Sampling for SamplingClient<H, DB> {
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
	) -> Result<(), Box<dyn std::error::Error>> {
		let id = ConfidenceId::app_confidence(app_id, nonce);

		let mut confidence = Confidence { samples: Vec::new(), commitments: commitments.clone() };

		confidence.set_sample(SAMPLES_PER_BLOB);

		let apps = vec![(app_id, nonce); SAMPLES_PER_BLOB];

		self.sample(&id, &mut confidence, &apps, &commitments).await
	}

	async fn sample_block<Header>(&self, header: &Header) -> Result<(), Box<dyn std::error::Error>>
	where
		Header: ExtendedHeader + HeaderT,
	{
		let id = ConfidenceId::block_confidence(header.hash().as_ref());
		let commitments = header.commitments().ok_or_else(|| {
			Box::new(std::io::Error::new(
				std::io::ErrorKind::NotFound,
				"Commitments not found in the header",
			)) as Box<dyn std::error::Error>
		})?;

		let mut confidence = Confidence { samples: Vec::new(), commitments: Vec::new() };

		confidence.set_sample(SAMPLES_PER_BLOB);

		let apps: Result<Vec<(u32, u32)>, Box<dyn std::error::Error>> = confidence
			.samples
			.iter()
			.map(|sample| {
				header
					.extension()
					.get_lookup(sample.position.y)
					.ok_or_else(|| {
						Box::new(std::io::Error::new(
							std::io::ErrorKind::InvalidData,
							"AppLookup not found for given sample position",
						)) as Box<dyn std::error::Error>
					})
					.map(|app_lookup| (app_lookup.app_id, app_lookup.nonce))
			})
			.collect();

		let apps = apps?;

		self.sample(&id, &mut confidence, &apps, &commitments).await?;

		let at = header.number();
		self.set_last_at::<<Header as HeaderT>::Number>(*at).await;
		Ok(())
	}
}

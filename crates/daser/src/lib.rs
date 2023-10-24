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

use melo_core_primitives::{
	confidence::{Confidence, ConfidenceId, Sample, SAMPLES_PER_BLOB},
	traits::DasKv,
	ExtendedHeader, Header, HeaderExtension,
};
use melo_das_network::{service::Service as DasNetworkService, Block, SegmentData};

pub struct SamplingClient {
	network: DasNetworkService,
	database: Box<dyn DasKv>,
}

impl SamplingClient {
	pub fn new(network: DasNetworkService, database: Box<dyn DasKv>) -> Self {
		SamplingClient { network, database }
	}

	pub async fn sample_application(
		&mut self,
		block_number: u32,
		app_id: u32,
		commitments: &Vec<KZGCommitment>,
	) -> Result<(), Box<dyn std::error::Error>> {
		let id = ConfidenceId::application_confidence(block_number, app_id);

		let mut confidence = Confidence { samples: Vec::new(), commitments: commitments.clone() };

		confidence.set_sample(SAMPLES_PER_BLOB).save(&id, &mut *self.database);

		let app_ids = vec![app_id; SAMPLES_PER_BLOB];

		self.start_sampling(&id, &mut confidence, block_number, &app_ids, &commitments)
			.await
	}

	pub async fn sample_block(
		&mut self,
		header: &Header,
	) -> Result<(), Box<dyn std::error::Error>> {
		let id = ConfidenceId::block_confidence(block_hash);
		let block_number = header.number;
		let block_hash = header.hash();
		let commitments = header.commitments()?;

		let mut confidence = Confidence { samples: Vec::new(), commitments: Vec::new() };

		confidence.set_sample(SAMPLES_PER_BLOB).save(&mut *self.database);

		let app_ids = confidence
			.samples
			.iter()
			.map(|sample| header_extension.get_app_id(sample.position.y))
			.collect();

		self.start_sampling(&id, &mut confidence, block_number, app_id, commitments).await
	}

	async fn start_sampling(
		&mut self,
		confidence_id: &ConfidenceId,
		confidence: &mut Confidence,
		block_number: BlockNumber,
		app_ids: &[u32],
		commitments: &[KZGCommitment],
	) -> Result<(), Box<dyn std::error::Error>> {
		if confidence.samples.len() != app_ids.len() {
			return Err(Box::new(std::io::Error::new(
				std::io::ErrorKind::InvalidData,
				"Length of samples does not match with app_ids.",
			)))
		}

		for sample in confidence.samples {
			if sample.position.y >= commitments.len() {
				return Err(Box::new(std::io::Error::new(
					std::io::ErrorKind::InvalidData,
					"Sample position y out of range for commitments.",
				)))
			}

			let values = self
				.network
				.get_value(KademliaKey::new(
					sample.key(block_number, app_ids[sample.position.y as usize]),
				)) // Assuming app_ids is indexed by position.y
				.await?;

			for value in values {
				if let Ok(segment_data) = SegmentData::decode(&mut &value[..]) {
					let segment =
						Segment { position: sample.position.clone(), content: segment_data };

					if let Err(e) = segment.checked() {
						return Err(Box::new(std::io::Error::new(
							std::io::ErrorKind::InvalidData,
							e,
						)))
					}

					if segment
						.verify(&self.kzg, &commitments[sample.position.y], segment.size())
						.is_ok()
					{
						sample.set_success();
						break
					}
				}
			}
		}

		confidence.save(&confidence_id, &mut *self.database);

		Ok(())
	}
}

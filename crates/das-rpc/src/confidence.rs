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

use jsonrpsee::{
	core::{async_trait, RpcResult},
	proc_macros::rpc,
};
use melo_core_primitives::reliability::{Reliability, ReliabilityId};

use futures::lock::Mutex;
use std::{marker::PhantomData, sync::Arc};

use melo_das_db::traits::DasKv;

pub use sc_rpc_api::DenyUnsafe;

/// Defines the Das API's functionalities.
#[rpc(client, server, namespace = "das")]
pub trait ConfidenceApi<DB, Hash> {
	#[method(name = "blockConfidence")]
	async fn block_confidence(&self, block_hash: Hash) -> RpcResult<Option<u32>>;

	#[method(name = "isAvailable")]
	async fn is_available(&self, block_hash: Hash) -> RpcResult<Option<bool>>;
}

pub struct Confidence<DB, Hash> {
	database: Arc<Mutex<DB>>,
	_marker: PhantomData<Hash>,
}

impl<DB, Hash> Confidence<DB, Hash>
where
	Hash: AsRef<[u8]> + Send + Sync + 'static,
	DB: DasKv + 'static,
{
	pub fn new(database: &Arc<Mutex<DB>>) -> Self {
		Self { database: database.clone(), _marker: PhantomData }
	}

	pub async fn confidence(&self, block_hash: Hash) -> Option<Reliability> {
		let confidence_id = ReliabilityId::block_confidence(block_hash.as_ref());

		// Lock the mutex asynchronously and await the guard
		let mut db = self.database.lock().await;

		confidence_id.get_confidence(&mut *db)
	}
}

#[async_trait]
impl<DB, Hash> ConfidenceApiServer<DB, Hash> for Confidence<DB, Hash>
where
	DB: DasKv + Send + Sync + 'static,
	Hash: AsRef<[u8]> + Send + Sync + 'static,
{
	async fn block_confidence(&self, block_hash: Hash) -> RpcResult<Option<u32>> {
		let confidence = self.confidence(block_hash).await;
		Ok(confidence.and_then(|c| c.value()))
	}

	async fn is_available(&self, block_hash: Hash) -> RpcResult<Option<bool>> {
		let confidence = self.confidence(block_hash).await;
		Ok(Some(confidence.map_or(false, |c| c.is_availability())))
	}
}

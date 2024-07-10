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

use crate::Error;

use jsonrpsee::{
	core::{async_trait, RpcResult},
	proc_macros::rpc,
};
use melo_core_primitives::reliability::{Reliability, ReliabilityId};

use futures::lock::Mutex;
use melo_daser::DasNetworkOperations;
use sp_core::Bytes;
use std::{marker::PhantomData, sync::Arc};

use melo_das_db::traits::DasKv;

/// Defines the Das API's functionalities.
#[rpc(client, server, namespace = "das")]
pub trait ConfidenceApi<DB, Hash, DN> {
	/// Returns the confidence of a block.
	/// If the block is not in the database, returns `None`.
	///
	/// # Arguments
	///
	/// * `block_hash` - A hash of the block.
	///
	/// # Returns
	///
	/// Returns the confidence of the block as an `Option<u32>`. If the block is not in the
	/// database, returns `None`.
	#[method(name = "blockConfidence")]
	async fn block_confidence(&self, block_hash: Hash) -> RpcResult<Option<u32>>;

	/// Returns whether the block is available.
	///
	/// # Arguments
	///
	/// * `block_hash` - A hash of the block.
	///
	/// # Returns
	///
	/// Returns whether the block is available as an `Option<bool>`. If the block is not in the
	/// database, returns `None`.
	#[method(name = "isAvailable")]
	async fn is_available(&self, block_hash: Hash) -> RpcResult<Option<bool>>;

	/// Removes records from the local node.
	///
	/// # Arguments
	///
	/// * `keys` - A vector of bytes representing the keys to remove.
	///
	/// # Returns
	///
	/// Returns `()` if the records were successfully removed.
	#[method(name = "removeRecords")]
	async fn remove_records(&self, keys: Vec<Bytes>) -> RpcResult<()>;

	#[method(name = "last")]
	async fn last(&self) -> RpcResult<Option<(u32, Bytes)>>;
}

/// The Das API's implementation.
pub struct Confidence<DB, Hash, DN> {
	database: Arc<Mutex<DB>>,
	das_network: Arc<DN>,
	_marker: PhantomData<Hash>,
}

impl<DB, Hash, DN> Confidence<DB, Hash, DN>
where
	Hash: AsRef<[u8]> + Send + Sync + 'static,
	DB: DasKv + 'static,
{
	/// Creates a new [`Confidence`] instance.
	pub fn new(database: &Arc<Mutex<DB>>, das_network: &Arc<DN>) -> Self {
		Self { database: database.clone(), das_network: das_network.clone(), _marker: PhantomData }
	}

	/// Returns the confidence of a block.
	pub async fn confidence(&self, block_hash: Hash) -> Option<Reliability> {
		let confidence_id = ReliabilityId::block_confidence(block_hash.as_ref());
		let mut db = self.database.lock().await;
		confidence_id.get_confidence(&mut *db)
	}

	pub async fn get_last(&self) -> Option<(Bytes, u32)> {
		let last_pr = ReliabilityId::get_last(&mut *self.database.lock().await);
		last_pr.map(|last| (Bytes::from(last.block_hash), last.block_num))
	}
}

#[async_trait]
impl<DB, Hash, DN> ConfidenceApiServer<DB, Hash, DN> for Confidence<DB, Hash, DN>
where
	DB: DasKv + Send + Sync + 'static,
	Hash: AsRef<[u8]> + Send + Sync + 'static,
	DN: DasNetworkOperations + Sync + Send + 'static + Clone,
{
	async fn block_confidence(&self, block_hash: Hash) -> RpcResult<Option<u32>> {
		let confidence = self.confidence(block_hash).await;
		Ok(confidence.and_then(|c| c.value()))
	}

	async fn is_available(&self, block_hash: Hash) -> RpcResult<Option<bool>> {
		let confidence = self.confidence(block_hash).await;
		Ok(Some(confidence.map_or(false, |c| c.is_availability())))
	}

	async fn remove_records(&self, keys: Vec<Bytes>) -> RpcResult<()> {
		let keys = keys.iter().map(|key| &**key).collect::<Vec<_>>();
		self.das_network
			.remove_records(keys)
			.await
			.map_err(|_| Error::FailedToRemoveRecords)?;

		Ok(())
	}

	async fn last(&self) -> RpcResult<Option<(u32, Bytes)>> {
		self.get_last()
			.await
			.map_or(Ok(None), |(hash, number)| Ok(Some((number, hash))))
	}
}

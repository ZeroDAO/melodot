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

use async_trait::async_trait;
pub use sc_network::{DhtEvent, KademliaKey, NetworkDHTProvider, NetworkSigner, NetworkStateInfo};
use std::fmt::Debug;

pub use melo_das_network::Service as DasDhtService;

/// `DasDht` trait provides an asynchronous interface for interacting with the DHT (Distributed Hash Table).
#[async_trait]
pub trait DasDht: Send + Debug + 'static {
    /// Asynchronously puts a key-value pair into the DHT.
    ///
    /// # Arguments
    /// 
    /// * `key` - The key to be inserted into the DHT.
    /// * `value` - The value associated with the provided key.
    ///
    /// # Returns
    /// 
    /// An `Option<()>` which is `Some(())` if the operation is successful and `None` otherwise.
	async fn put_value_to_dht(&mut self, key: KademliaKey, value: Vec<u8>) -> Option<()>;
}

#[async_trait]
impl DasDht for DasDhtService {
    /// Implementation of the `put_value_to_dht` method for `DasDhtService`.
	async fn put_value_to_dht(&mut self, key: KademliaKey, value: Vec<u8>) -> Option<()> {
		DasDhtService::put_value_to_dht(self, key, value).await
	}
}

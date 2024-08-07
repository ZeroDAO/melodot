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

use crate::{Command, KademliaKey};
use anyhow::Context;
use futures::{
	channel::{mpsc, oneshot},
	future::join_all,
	SinkExt,
};
use libp2p::{
	futures,
	kad::{Quorum, Record},
	Multiaddr, PeerId,
	// libp2p_kad::record::Key
};
use std::{fmt::Debug, time::Duration};

/// `Service` serves as an intermediary to interact with the Worker, handling requests and
/// facilitating communication. It mainly operates on the message passing mechanism between service
/// and worker.
#[derive(Clone)]
pub struct Service {
	// Channel sender to send messages to the worker.
	to_worker: mpsc::Sender<Command>,
	// The maximum number of parallel requests to the worker.
	parallel_limit: usize,
}

impl Debug for Service {
	/// Provides a human-readable representation of the Service, useful for debugging.
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_tuple("DasNetworkService").finish()
	}
}

impl Service {
	/// Constructs a new `Service` instance with a given channel to communicate with the worker.
	pub(crate) fn new(to_worker: mpsc::Sender<Command>, parallel_limit: usize) -> Self {
		Self { to_worker, parallel_limit }
	}

	/// Starts listening on the given multi-address.
	pub async fn start_listening(&self, addr: Multiaddr) -> anyhow::Result<()> {
		let (sender, receiver) = oneshot::channel();
		self.to_worker.clone().send(Command::StartListening { addr, sender }).await?;
		receiver.await.context("Failed receiving start listening response")?
	}

	/// Adds a peer's address to the kademlia instance.
	pub async fn add_address(&self, peer_id: PeerId, peer_addr: Multiaddr) -> anyhow::Result<()> {
		let (sender, receiver) = oneshot::channel();
		self.to_worker
			.clone()
			.send(Command::AddAddress { peer_id, peer_addr, sender })
			.await?;
		receiver.await.context("Failed receiving add address response")?
	}

	/// Asynchronously gets the value corresponding to `key` from the Kademlia network. This will return a vector 
	/// of multiple results, which need to be verified manually.
	pub async fn get_value(&self, key: KademliaKey) -> anyhow::Result<Vec<Vec<u8>>> {
		let records = self.get_kad_record(key).await?;
		Ok(records.into_iter().map(|r| r.value).collect())
	}

	/// Asynchronously puts data into the Kademlia network.
	pub async fn put_value(&self, key: KademliaKey, value: Vec<u8>) -> anyhow::Result<()> {
		let record = Record::new(key, value);
		self.put_kad_record(record, Quorum::All).await
	}

	/// Asynchronously gets the values corresponding to multiple `keys` from the Kademlia network. This will return 
	/// a vector of multiple results, which need to be verified manually.
	pub async fn get_values(
		&self,
		keys: &[KademliaKey],
	) -> anyhow::Result<Vec<Option<Vec<Vec<u8>>>>> {
		let mut results = Vec::with_capacity(keys.len());

		for chunk in keys.chunks(self.parallel_limit) {
			let futures = chunk.iter().map(|key| self.get_value(key.clone()));
			let chunk_results = join_all(futures).await;
			for res in chunk_results {
				match res {
					Ok(v) => results.push(Some(v)),
					Err(_) => results.push(None),
				}
			}
		}

		Ok(results)
	}

	/// Asynchronously puts multiple data into the Kademlia network.
	pub async fn put_values(
		&self,
		keys_and_values: Vec<(KademliaKey, Vec<u8>)>,
	) -> anyhow::Result<()> {
		let futures = keys_and_values.into_iter().map(|(key, value)| self.put_value(key, value));
		join_all(futures).await;
		Ok(())
	}

	/// Queries the DHT for a record.
	pub async fn get_kad_record(&self, key: KademliaKey) -> anyhow::Result<Vec<Record>> {
		let (sender, receiver) = oneshot::channel();
		self.to_worker.clone().send(Command::GetKadRecord { key, sender }).await?;
		receiver.await.context("Failed receiving get record response")?
	}

	/// Puts a record into the DHT.
	pub async fn put_kad_record(&self, record: Record, quorum: Quorum) -> anyhow::Result<()> {
		let (sender, receiver) = oneshot::channel();
		self.to_worker
			.clone()
			.send(Command::PutKadRecord { record, quorum, sender })
			.await?;
		receiver.await.context("Failed receiving put record response")?
	}

	/// Asynchronously removes the values corresponding to multiple `keys` from the local storage, including values stored 
	/// as storage nodes.
	pub async fn remove_records(&self, keys: &[KademliaKey]) -> anyhow::Result<()> {
		let (sender, receiver) = oneshot::channel();
		self.to_worker
			.clone()
			.send(Command::RemoveRecords { keys: keys.to_vec(), sender })
			.await?;
		receiver.await.context("Failed receiving remove records response")?
	}
}

/// Configuration for the DAS network service.
#[derive(Clone, Debug)]
pub struct DasNetworkConfig {
	/// The IP address to listen on.
	pub listen_addr: String,
	/// The port to listen on.
	pub listen_port: u16,
	/// List of bootstrap nodes to connect to.
	pub bootstrap_nodes: Vec<String>,
	/// Maximum number of retries when connecting to a node.
	pub max_retries: usize,
	/// Delay between retries when connecting to a node.
	pub retry_delay: Duration,
	/// Timeout for bootstrapping the network.
	pub bootstrap_timeout: Duration,
	/// Maximum number of parallel connections to maintain.
	pub parallel_limit: usize,
}

impl Default for DasNetworkConfig {
	fn default() -> Self {
		DasNetworkConfig {
			listen_addr: "0.0.0.0".to_string(),
			listen_port: 4417,
			bootstrap_nodes: vec![],
			max_retries: 3,
			retry_delay: Duration::from_secs(5),
			bootstrap_timeout: Duration::from_secs(60),
			parallel_limit: 10,
		}
	}
}

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
use async_trait::async_trait;
use futures::{
	channel::{mpsc, oneshot},
	future::FutureExt,
	SinkExt,
};
use libp2p::{
	futures,
	kad::{record, Quorum, Record},
	Multiaddr, PeerId,
};
use rand::seq::SliceRandom;
use std::{fmt::Debug, time::Duration};

/// `Service` serves as an intermediary to interact with the Worker, handling requests and
/// facilitating communication. It mainly operates on the message passing mechanism between service
/// and worker.
#[derive(Clone)]
pub struct Service {
	// Channel sender to send messages to the worker.
	to_worker: mpsc::Sender<Command>,
}

impl Debug for Service {
	/// Provides a human-readable representation of the Service, useful for debugging.
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_tuple("DasNetworkService").finish()
	}
}

impl Service {
	/// Constructs a new `Service` instance with a given channel to communicate with the worker.
	pub(crate) fn new(to_worker: mpsc::Sender<Command>) -> Self {
		Self { to_worker }
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

	/// Bootstraps the kademlia protocol.
	pub async fn bootstrap(&self) -> anyhow::Result<()> {
		let (sender, receiver) = oneshot::channel();
		self.to_worker.clone().send(Command::Bootstrap { sender }).await?;
		receiver.await.context("Failed receiving bootstrap response")?
	}

	// 需要接受验证函数并传输到底层，处理节点声誉
	pub async fn get_value(&self, key: KademliaKey) -> anyhow::Result<Vec<Vec<u8>>> {
		let records = self.get_kad_record(key).await?;
		Ok(records.into_iter().map(|r| r.value).collect())
	}

	pub async fn put_value(&self, key: KademliaKey, value: Vec<u8>) -> anyhow::Result<()> {
		let record = Record::new(key as record::Key, value);
		self.put_kad_record(record, Quorum::All).await
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
}

#[async_trait]
pub trait DasNetworkDiscovery {
	async fn init(&self, config: &DasNetworkConfig) -> anyhow::Result<()>;
	async fn connect_to_bootstrap_node(&self, addr_str: &String, config: &DasNetworkConfig) -> anyhow::Result<()>;
}

pub struct DasNetworkConfig {
    pub listen_addr: String,
    pub listen_port: u16,
    pub bootstrap_nodes: Vec<String>,
    pub max_retries: usize,
    pub retry_delay: Duration,
    pub bootstrap_timeout: Duration,
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
        }
    }
}

#[async_trait]
impl DasNetworkDiscovery for Service {
	async fn init(&self, config: &DasNetworkConfig) -> anyhow::Result<()> {
		let address_string = format!("/ip4/{}/tcp/{}", config.listen_addr, config.listen_port);
        let listen_address: Multiaddr = address_string.parse().expect("Invalid address format");

		self.start_listening(listen_address).await?;
		// Shuffle the nodes for randomness, ensuring all nodes don't connect to the same bootstrap
		// node simultaneously.
		let mut rng = rand::thread_rng();
		let mut shuffled_nodes = config.bootstrap_nodes.clone();
		shuffled_nodes.shuffle(&mut rng);

		// Connect to a limited number of bootstrap nodes, for example, up to 3.
		for addr in shuffled_nodes.iter().take(3) {
			self.connect_to_bootstrap_node(addr, config).await?;
		}

		// Initiate bootstrap with a timeout
		let bootstrap_fut = self.bootstrap().boxed();
		let timeout_fut = tokio::time::sleep(config.bootstrap_timeout).boxed();
		futures::pin_mut!(bootstrap_fut, timeout_fut);

		match futures::future::select(bootstrap_fut, timeout_fut).await {
			futures::future::Either::Left((Ok(_), _)) => {
				log::info!("Successfully bootstrapped to the network.");
				Ok(())
			},
			futures::future::Either::Left((Err(e), _)) => {
				log::error!("Failed to bootstrap to the network. Error: {}", e);
				Err(e)
			},
			futures::future::Either::Right(_) => {
				log::error!(
					"Bootstrap to the network timed out after {:?}",
					config.bootstrap_timeout
				);
				Err(anyhow::anyhow!(
					"Bootstrap to the network timed out after {:?}",
					config.bootstrap_timeout
				))
			},
		}
	}

	async fn connect_to_bootstrap_node(&self, addr_str: &String, config: &DasNetworkConfig) -> anyhow::Result<()> {
		let addr: Multiaddr = addr_str.parse().context("Failed parsing bootstrap node address")?;
		let peer_id = match addr.iter().last() {
			Some(libp2p::multiaddr::Protocol::P2p(hash)) => PeerId::from_multihash(hash).expect("Invalid peer multiaddress."),
			_ => return Err(anyhow::anyhow!("Failed extracting PeerId from address")),
		};
	
		for i in 0..config.max_retries {
			match self.add_address(peer_id.clone(), addr.clone()).await {
				Ok(_) => {
					log::info!("Successfully connected to bootstrap node: {}", addr_str);
					break;
				},
				Err(e) if i < config.max_retries - 1 => {
					log::warn!(
						"Failed to connect to bootstrap node: {}. Retry {}/{} in {:?}",
						addr_str,
						i + 1,
						config.max_retries,
						config.retry_delay
					);
					tokio::time::sleep(config.retry_delay).await;
				},
				Err(e) => {
					log::error!(
						"Failed to connect to bootstrap node: {} after {} retries. Error: {}",
						addr_str, config.max_retries, e
					);
					return Err(e);
				}
			}
		}
	
		Ok(())
	}
	
}


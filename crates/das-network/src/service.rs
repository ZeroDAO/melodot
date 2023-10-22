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
	SinkExt,
};
use libp2p::{
	futures,
	kad::{Quorum, Record},
	Multiaddr, PeerId,
};
use std::fmt::Debug;

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

	// /// Puts a key-value pair to the DHT (Distributed Hash Table).
	// /// Sends a message to the worker to perform the DHT insertion and awaits its acknowledgment.
	// ///
	// /// # Parameters
	// /// - `key`: The `KademliaKey` under which the value will be stored in the DHT.
	// /// - `value`: The actual data to be stored in the DHT.
	// ///
	// /// # Returns
	// /// - An `Option<()>` signaling the success or failure of the operation. The `None` variant
	// ///   indicates a failure.
	// pub async fn put_value_to_dht(&mut self, key: KademliaKey, value: Vec<u8>) -> Option<()> {
	// 	let record = Record::new(key.into(), value);
	// 	self.put_record(record, Quorum::One).await
	// }

	// /// Returns a stream of events from the worker.
	// /// Sends a message to the worker to start streaming events and awaits its acknowledgment.
	// pub async fn events_stream(&mut self) -> ReceiverStream<BehaviourEvent> {
	// 	// Create a one-shot channel for immediate communication.
	// 	let (sender, receiver) = mpsc::channel(1000);

	// 	// Send a request to the worker to start streaming events.
	// 	self.to_worker.send(Command::Stream(sender)).await.ok()?;

	// 	// Wait for the worker's response.
	// 	ReceiverStream::new(receiver)
	// }
}

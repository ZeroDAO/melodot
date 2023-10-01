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

use futures::{
	channel::{mpsc, oneshot},
	SinkExt,
};
use std::fmt::Debug;

use crate::{KademliaKey, ServicetoWorkerMsg};

/// `Service` serves as an intermediary to interact with the Worker, handling requests and facilitating communication.
/// It mainly operates on the message passing mechanism between service and worker.
#[derive(Clone)]
pub struct Service {
	// Channel sender to send messages to the worker.
	to_worker: mpsc::Sender<ServicetoWorkerMsg>,
}

impl Debug for Service {
	/// Provides a human-readable representation of the Service, useful for debugging.
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_tuple("DasNetworkService").finish()
	}
}

impl Service {
	/// Constructs a new `Service` instance with a given channel to communicate with the worker.
	pub(crate) fn new(to_worker: mpsc::Sender<ServicetoWorkerMsg>) -> Self {
		Self { to_worker }
	}

	/// Puts a key-value pair to the DHT (Distributed Hash Table).
	/// Sends a message to the worker to perform the DHT insertion and awaits its acknowledgment.
	///
	/// # Parameters
	/// - `key`: The `KademliaKey` under which the value will be stored in the DHT.
	/// - `value`: The actual data to be stored in the DHT.
	///
	/// # Returns
	/// - An `Option<()>` signaling the success or failure of the operation. 
	///   The `None` variant indicates a failure.
	pub async fn put_value_to_dht(&mut self, key: KademliaKey, value: Vec<u8>) -> Option<()> {
		// Create a one-shot channel for immediate communication.
		let (tx, rx) = oneshot::channel();

		// Send a request to the worker to put the key-value pair to the DHT.
		self.to_worker
			.send(ServicetoWorkerMsg::PutValueToDht(key, value, tx))
			.await
			.ok()?;

		// Wait for the worker's response.
		rx.await.ok().flatten()
	}
}

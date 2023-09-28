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

/// Service to interact with the [`crate::Worker`].
#[derive(Clone)]
pub struct Service {
	to_worker: mpsc::Sender<ServicetoWorkerMsg>,
}

impl Debug for Service {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_tuple("DasNetworkService").finish()
	}
}

/// A [`Service`] allows to interact with a [`crate::Worker`], e.g. by querying the
/// [`crate::Worker`]'s local address cache for a given [`AuthorityId`].
impl Service {
	pub(crate) fn new(to_worker: mpsc::Sender<ServicetoWorkerMsg>) -> Self {
		Self { to_worker }
	}

	pub async fn put_value_to_dht(&mut self, key: KademliaKey, value: Vec<u8>) -> Option<()> {
		let (tx, rx) = oneshot::channel();

		self.to_worker
			.send(ServicetoWorkerMsg::PutValueToDht(key, value, tx))
			.await
			.ok()?;

		rx.await.ok().flatten()
	}
}

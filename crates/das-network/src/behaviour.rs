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

use libp2p::{core::PeerId, kad::store::MemoryStore};

use libp2p::{
	identify::{Behaviour as Identify, Config as IdentifyConfig, Event as IdentifyEvent},
	kad::{Kademlia, KademliaConfig, KademliaEvent},
	ping::{Behaviour as Ping, Event as PingEvent},
};
use libp2p::swarm::NetworkBehaviour;
use derive_more::From;

pub struct BehaviorConfig {
	/// Identity keypair of a node used for authenticated connections.
	pub peer_id: PeerId,
	/// The configuration for the [`Identify`] behaviour.
	pub identify: IdentifyConfig,
	/// The configuration for the [`Kademlia`] behaviour.
	pub kademlia: KademliaConfig,
	/// The configuration for the [`kad_store`] behaviour.
	pub kad_store: MemoryStore,
}

#[derive(NetworkBehaviour)]
#[behaviour(out_event = "BehaviourEvent")]
#[behaviour(event_process = false)]
pub struct Behavior {
	pub kademlia: Kademlia<MemoryStore>,
	pub identify: Identify,
	pub ping: Ping,
}

impl Behavior {
	pub fn new(config: BehaviorConfig) -> Self {
		let kademlia = Kademlia::with_config(config.peer_id, config.kad_store, config.kademlia);

		Self { identify: Identify::new(config.identify), kademlia, ping: Ping::default() }
	}
}

#[derive(Debug, From)]
pub enum BehaviourEvent {
	Identify(IdentifyEvent),
	Kademlia(KademliaEvent),
	Ping(PingEvent),
}

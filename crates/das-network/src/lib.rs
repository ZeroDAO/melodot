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

pub use node_primitives::AccountId;
pub use sc_network::{KademliaKey, NetworkDHTProvider, NetworkSigner, NetworkStateInfo};

pub mod dht_work;
pub mod tx_pool_listener;

pub trait NetworkProvider: NetworkDHTProvider + NetworkStateInfo + NetworkSigner {}
impl<T> NetworkProvider for T where T: NetworkDHTProvider + NetworkStateInfo + NetworkSigner {}

pub use melo_core_primitives::{
	Sidercar, SidercarMetadata,
	SidercarStatus,
};
use sp_core::H256;

pub fn sidercar_kademlia_key(sidercar: &Sidercar) -> KademliaKey {
	KademliaKey::from(Vec::from(sidercar.id()))
}

pub fn kademlia_key_from_sidercar_id(sidercar_id: &H256) -> KademliaKey {
	KademliaKey::from(Vec::from(&sidercar_id[..]))
}
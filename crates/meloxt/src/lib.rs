// Copyright 2023 ZeroDAO

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at

//     http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use subxt::{
    Config, PolkadotConfig,
    config::substrate::BlakeTwo256,
    utils::{AccountId32, MultiAddress, MultiSignature},
    OnlineClient, 
};
use subxt_signer::sr25519::dev::{self};
use subxt_signer::sr25519::Keypair;

#[subxt::subxt(runtime_metadata_path = "melodot_metadata.scale")]
pub mod melodot {}

pub mod header;
use header::MelodotHeader;

mod log;
pub use log::init_logger;

mod helper;
pub use helper::*;
pub enum MeloConfig {}

pub type Signature = MultiSignature;
pub type AccountId = AccountId32;
pub type AccountIndex = u32;
pub type Address = MultiAddress<AccountId, AccountIndex>;

impl Config for MeloConfig {
    type Hash = H256;
    type AccountId = AccountId;
    type Address = Address;
    type Signature = Signature;
    type Hasher = BlakeTwo256;
    type Header = MelodotHeader<u32, BlakeTwo256>;
    type ExtrinsicParams = <PolkadotConfig as Config>::ExtrinsicParams;
}

pub struct Client {
    pub api: OnlineClient<MeloConfig>,
    pub signer: Keypair,
}

impl Client {
    pub fn set_signer(&mut self, signer: Keypair) {
        self.signer = signer;
    }

    pub fn set_client(&mut self, api: OnlineClient<MeloConfig>) {
        self.api = api;
    }
}

pub struct ClientBuilder {
    pub url: String,
    pub signer: Keypair,
}

impl ClientBuilder {
    pub fn new(url: &str, signer: Keypair) -> Self {
        Self {
            url: url.to_string(),
            signer,
        }
    }

    pub async fn build(&self) -> Result<Client, Box<dyn std::error::Error>> {
        let api = OnlineClient::<MeloConfig>::from_url(&self.url).await?;
        Ok(Client {
            api,
            signer: self.signer.clone(),
        })
    }
}

impl Default for ClientBuilder {
    fn default() -> Self {
        Self {
            url: "ws://127.0.0.1:9944".to_owned(),
            signer: dev::alice(),
        }
    }
}
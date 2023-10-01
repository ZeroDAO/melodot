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

// Load the runtime metadata from the provided path.
#[subxt::subxt(runtime_metadata_path = "melodot_metadata.scale")]
pub mod melodot {}

pub mod header;
use header::MelodotHeader;

mod log;
pub use log::init_logger;

mod helper;
pub use helper::*;

/// Configuration enum for Melo blockchain.
pub enum MeloConfig {}

// Define aliases for commonly used types.
pub type Signature = MultiSignature;
pub type AccountId = AccountId32;
pub type AccountIndex = u32;
pub type Address = MultiAddress<AccountId, AccountIndex>;

// Implement the `Config` trait for `MeloConfig`, mapping Melo-specific types to the substrate types.
impl Config for MeloConfig {
    type Hash = H256;
    type AccountId = AccountId;
    type Address = Address;
    type Signature = Signature;
    type Hasher = BlakeTwo256;
    type Header = MelodotHeader<u32, BlakeTwo256>;
    type ExtrinsicParams = <PolkadotConfig as Config>::ExtrinsicParams;
}

/// Client structure containing the API for blockchain interactions and a signer for transactions.
pub struct Client {
    pub api: OnlineClient<MeloConfig>,
    pub signer: Keypair,
}

impl Client {
    /// Update the signer for the client.
    pub fn set_signer(&mut self, signer: Keypair) {
        self.signer = signer;
    }

    /// Update the API client.
    pub fn set_client(&mut self, api: OnlineClient<MeloConfig>) {
        self.api = api;
    }
}

/// A builder pattern for creating a `Client` instance.
pub struct ClientBuilder {
    pub url: String,
    pub signer: Keypair,
}

impl ClientBuilder {
    /// Constructor for `ClientBuilder`.
    pub fn new(url: &str, signer: Keypair) -> Self {
        Self {
            url: url.to_string(),
            signer,
        }
    }

    /// Asynchronously build and return a `Client` instance.
    pub async fn build(&self) -> Result<Client, Box<dyn std::error::Error>> {
        let api = OnlineClient::<MeloConfig>::from_url(&self.url).await?;
        Ok(Client {
            api,
            signer: self.signer.clone(),
        })
    }
}

// Default implementation for `ClientBuilder`.
impl Default for ClientBuilder {
    fn default() -> Self {
        Self {
            url: "ws://127.0.0.1:9944".to_owned(),
            signer: dev::alice(),
        }
    }
}

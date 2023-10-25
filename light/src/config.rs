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

use clap::{App, Arg};
use melo_das_network::DasNetworkConfig;

pub enum NetworkConfigType {
    Dev,
    Test,
}

pub struct Config {
    pub rpc_url: String,
    pub network_config: DasNetworkConfig,
}

impl Config {
    pub fn from_config_type(config_type: NetworkConfigType) -> Self {
        match config_type {
            NetworkConfigType::Dev => Config {
                rpc_url: "YOUR_DEV_RPC_URL_HERE".to_string(),
                network_config: DasNetworkConfig {
                    bootstrap_nodes: vec![],
                    ..DasNetworkConfig::default(),
                },
            },
            NetworkConfigType::Test => Config {
                rpc_url: "YOUR_TEST_RPC_URL_HERE".to_string(),
                network_config: DasNetworkConfig {
                    bootstrap_nodes: vec![],
                    ..DasNetworkConfig::default(),
                },
            },
        }
    }
}

pub fn parse_args() -> Config {
    let matches = App::new("Light Node")
        .arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .value_name("TYPE")
                .possible_values(&["dev", "test"])
                .default_value("dev")
                .help("Specify which network configuration to use"),
        )
        .get_matches();

    let config_type = match matches.value_of("config").unwrap() {
        "dev" => NetworkConfigType::Dev,
        "test" => NetworkConfigType::Test,
        _ => panic!("Unknown config type"),
    };

    Config::from_config_type(config_type)
}

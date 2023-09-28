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

use std::process::Stdio;
use anyhow::{ensure, Result};
use tokio::process::Command as TokioCommand;

#[tokio::main]
async fn main() -> Result<()> {
    let examples = fetch_all_examples().await?;
    for example in examples.iter() {
        println!("Running example: {}", example);
        run_example(example).await?;
    }
    Ok(())
}

async fn run_example(example: &str) -> Result<()> {
    let status = TokioCommand::new("cargo")
        .args(&["run", "--release", "--example", example])
        .status()
        .await?;

    ensure!(status.success(), format!("Example {} failed", example));
    Ok(())
}

async fn fetch_all_examples() -> Result<Vec<String>> {
    let output = tokio::task::spawn_blocking(move || {
        std::process::Command::new("cargo")
            .args(&["run", "--release", "--example"])
            .stderr(Stdio::piped())
            .output()
    })
    .await??;

    let lines = String::from_utf8(output.stderr)?
        .lines()
        .skip(2) // Skip the first 2 lines which might be info or error messages.
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .collect();

    Ok(lines)
}

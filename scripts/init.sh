#!/usr/bin/env bash
# This script is meant to be run on Unix/Linux based systems
set -e

echo "*** Initializing WASM build environment"

# if [ -z $CI_PROJECT_NAME ] ; then
#    rustup update nightly
#    rustup update stable
# fi

rustup default nightly-2023-08-26

rustup target add wasm32-unknown-unknown --toolchain nightly-2023-08-26

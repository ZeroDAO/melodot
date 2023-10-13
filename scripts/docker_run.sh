#!/usr/bin/env bash

set -e

echo "*** Melodot ***"

cd $(dirname ${BASH_SOURCE[0]})/..

# Ensure the .local directory exists
if [ ! -d "./.local" ]; then
  mkdir ./.local
fi

# Get the arguments passed to the script
cmd=$1
crate=$2

if [ "$cmd" == "new" ]; then
  docker exec -it melodot bash
  exit 0
fi

docker compose down --remove-orphans

# Check if the crate parameter is provided
if [ -z "$crate" ]; then
  docker compose run --rm --name melodot --service-ports dev $@
  exit 0
fi

# Define a function to execute different commands
execute_command() {
  crate_name="$1"
  cd "crates/$crate_name"
  case "$cmd" in
    "build")
      cargo build
      ;;
    "test")
      cargo test
      ;;
    # You can add more command options here
    *)
      echo "Unknown command: $cmd"
      exit 1
      ;;
  esac
}

# Call the function to execute the command
execute_command "$crate"
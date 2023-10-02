#!/usr/bin/env bash

set -e

echo "*** Melodot ***"

cd $(dirname ${BASH_SOURCE[0]})/..

docker-compose down --remove-orphans

# Get the arguments passed to the script
cmd=$1
crate=$2

# Check if the crate parameter is provided
if [ -z "$crate" ]; then
  docker-compose run --rm --service-ports dev $@
else
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

  read -p "$cmd completed. Press Enter to exit..."
fi

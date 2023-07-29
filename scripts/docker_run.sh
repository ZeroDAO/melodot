#!/usr/bin/env bash

set -e

echo "*** Melodot ***"

cd $(dirname ${BASH_SOURCE[0]})/..

docker-compose down --remove-orphans
docker-compose run --rm --service-ports dev $@

# Get the arguments passed to the script
command=$1
crate=$2

# Check if the crate parameter is provided
if [ -z "$crate" ]; then
  echo "Please provide the crate name."
else
  # Define a function to execute different commands
  execute_command() {
    crate_name="$1"
    cd "crates/$crate_name"
    case "$command" in
      "test")
        cargo test
        ;;
      "build")
        cargo build
        ;;
      # You can add more command options here
      *)
        echo "Unknown command: $command"
        exit 1
        ;;
    esac
  }

  # Call the function to execute the command
  execute_command "$crate"

  read -p "Tests completed. Press Enter to exit..."
fi

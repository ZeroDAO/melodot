#!/bin/bash

# URL for the data download
data_url="https://seq.ceremony.ethereum.org/info/current_state"

# Function to check if the input value is a valid g1_powers value
is_valid_g1_powers() {
  local g1_powers_list=("4096" "8192" "16384" "32768")
  for power in "${g1_powers_list[@]}"; do
    if [[ "$power" == "$1" ]]; then
      return 0
    fi
  done
  return 1
}

# Function to install jq based on the package manager
install_jq() {
  if command -v apt-get &> /dev/null; then
    sudo apt-get update
    sudo apt-get install -y jq
  elif command -v yum &> /dev/null; then
    sudo yum install -y jq
  elif command -v brew &> /dev/null; then
    brew install jq
  else
    echo "Could not find a supported package manager to install jq. Please install jq manually and try again."
    exit 1
  fi
}

# Check if the correct number of arguments is provided
if [[ $# -ne 1 ]]; then
  echo "Usage: $0 <g1_powers>"
  exit 1
fi

# Check if jq is installed, and if not, install it
if ! command -v jq &> /dev/null; then
  echo "jq is not installed. Installing jq..."
  install_jq || exit 1
fi

# Check if the provided g1_powers value is valid
if ! is_valid_g1_powers "$1"; then
  echo "Invalid g1_powers value. Valid values are: [4096, 8192, 16384, 32768]"
  exit 1
fi

# Assign g1_powers variable
g1_powers="$1"

# Determine the index corresponding to the provided g1_powers value
g1_powers_index=0
case "$g1_powers" in
  "8192") g1_powers_index=1 ;;
  "16384") g1_powers_index=2 ;;
  "32768") g1_powers_index=3 ;;
  # For "4096", the index is already 0 (default value)
esac

# Function to show a spinner animation with download progress
show_spinner_with_progress() {
  local spinner_chars="/-\|"
  local spinner_length=${#spinner_chars}
  local current_spinner_index=0
  local delay=0.05  # Adjust the delay to control animation speed
  local progress_file="curl_progress.tmp"

  local total_size=224998000  # Default value to avoid division by zero

  # Get the total file size from the headers before starting the spinner
  file_size_header=$(curl -sI "$data_url" | grep -i 'Content-Length' | awk '{print $2}' | tr -d '\r')
  if [[ -n "$file_size_header" ]]; then
    total_size="$file_size_header"
  fi

  while true; do
    local current_size=$(stat -c %s "$progress_file" 2>/dev/null || echo 0)

    # Avoid division by zero by using a default value if total_size is still 0
    local progress=$((total_size == 0 ? 0 : current_size * 100 / total_size))

    # Generate the progress bar using a loop
    local bar_length=$((progress / 2))
    local bar=""
    for ((i = 0; i < bar_length; i++)); do
      bar+="="
    done

    local spinner_char="${spinner_chars:$current_spinner_index:1}"
    current_spinner_index=$(( (current_spinner_index + 1) % spinner_length ))

    printf "\rDownloading data: [%-50s] %3d%% %s" "$bar" "$progress" "$spinner_char"

    # Wait for a short duration before updating the progress and animation
    sleep "$delay"
  done
}

# Display the custom message before starting the download
echo "Downloading data from: $data_url"
echo "This may take approximately 10 minutes. Please do not close this window."

# Replace g1_powers and g1_powers_index in the command and execute it within a subshell
(
  # Start the spinner animation with download progress while curl is running
  show_spinner_with_progress & spinner_pid=$!

  # Download the data using curl and suppress progress output
  curl -L "$data_url" > curl_progress.tmp 2>/dev/null

  # Stop the spinner animation once curl is finished
  kill -SIGTERM "$spinner_pid"

  # Show a newline to ensure the next line starts correctly
  echo

  # Get the final size of the downloaded data
  file_size=$(stat -c %s curl_progress.tmp)

  # Extract the required data and create the binary file
  jq ".transcripts[$g1_powers_index].powersOfTau" < curl_progress.tmp | jq -r '.G1Powers + .G2Powers | map(.[2:]) | join("")' | xxd -r -p - "eth-public-parameters-$g1_powers.bin"

  # Clean up temporary file
  rm curl_progress.tmp
)

#!/usr/bin/env bash
# -----------------------------------------------------------------------------
# Bootstrap and configure multiple threshold nodes for local development.
#
# This script will:
#   1. Generate N node identities & config files into the `keys/` directory.
#   2. Update each node's config to peer with the other N-1 nodes.
#   3. Generate a docker-compose.yaml file to run the N nodes.
#
# Usage example:
#   ./setup_nodes.sh          # generates 5 nodes
#   ./setup_nodes.sh 8             # generates 8 nodes
# -----------------------------------------------------------------------------

set -euo pipefail

# --------------------------- Configuration -----------------------------------
NUM_NODES=${1:-5}
BASE_DIR="test_artifacts/"

CLI_CMD=(cargo run --quiet --bin cli)
BASE_GRPC_PORT=50057
DOCKER_COMPOSE_FILE="docker-compose.yaml"
KEY_PASSWORD="supersecret"

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)

if [[ -f "$SCRIPT_DIR/Cargo.toml" ]]; then
  PROJECT_ROOT="$SCRIPT_DIR"
elif [[ -f "$SCRIPT_DIR/../Cargo.toml" ]]; then
  PROJECT_ROOT="$(realpath "$SCRIPT_DIR/..")"
else
  echo "ERROR: Could not locate Cargo.toml relative to $SCRIPT_DIR" >&2
  exit 1
fi

cd "$PROJECT_ROOT"

if [ -d "$BASE_DIR" ]; then
    echo "Removing existing '$BASE_DIR' directory..."
    rm -rf "$BASE_DIR"
fi

find . -name "*test_artifacts*test_artifacts*" -type f -delete 2>/dev/null || true
find . -name "*node_*node_*" -type f -delete 2>/dev/null || true

mkdir -p "$BASE_DIR"

CONFIGS_DIR="$BASE_DIR/test_${NUM_NODES}_nodes"

declare -a PEER_IDS
declare -a CONFIG_PATHS

# --------------------------- Generation loop ---------------------------------
echo "Generating $NUM_NODES node configurations in '$BASE_DIR'..."
for i in $(seq 1 "$NUM_NODES"); do
  node_name="node_${i}"
  node_dir="$CONFIGS_DIR/$node_name"
  mkdir -p "$node_dir"

  echo "  → $node_name"

  OUTPUT=$(KEY_PASSWORD="$KEY_PASSWORD" \
           "${CLI_CMD[@]}" setup \
             --output-dir "$(realpath "$node_dir")" \
             --file-name "$node_name")

  PEER_ID=$(echo "$OUTPUT" | sed -n -E 's/.*peer id ([A-Za-z0-9]+).*/\1/p')
  CONFIG_PATH=$(echo "$OUTPUT" | sed -n -E 's/.*config file here: (.*)$/\1/p')

  if [[ -z "$PEER_ID" || -z "$CONFIG_PATH" ]]; then
    echo "Failed to parse peer-id or config path from CLI output, aborting." >&2
    exit 1
  fi

  PEER_IDS[$i]="$PEER_ID"
  CONFIG_PATHS[$i]="$CONFIG_PATH"
done

# --------------------------- Patching configs --------------------------------
echo -e "\nPatching each node's allowed_peers list and signer configuration..."
ALL_PEERS=$(IFS=','; echo "${PEER_IDS[*]}")

MAX_SIGNERS=$NUM_NODES
MIN_SIGNERS=$(( (NUM_NODES * 2) / 3 ))

echo "Setting min_signers: $MIN_SIGNERS, max_signers: $MAX_SIGNERS"

for i in $(seq 1 "$NUM_NODES"); do
  CONFIG_PATH="${CONFIG_PATHS[$i]}"

  CONFIG_PATH="$CONFIG_PATH" \
  SELF_INDEX="$i" \
  ALL_PEERS="$ALL_PEERS" \
  MIN_SIGNERS="$MIN_SIGNERS" \
  MAX_SIGNERS="$MAX_SIGNERS" \
  python3 - <<'PY'
import os, pathlib, yaml, sys

config_path = pathlib.Path(os.environ['CONFIG_PATH'])
self_index   = int(os.environ['SELF_INDEX'])
all_peers    = os.environ['ALL_PEERS'].split(',')
min_signers  = int(os.environ['MIN_SIGNERS'])
max_signers  = int(os.environ['MAX_SIGNERS'])

try:
    data = yaml.safe_load(config_path.read_text()) or {}
except FileNotFoundError:
    sys.stderr.write(f"Config file not found: {config_path}\n")
    sys.exit(1)

allowed = []
for idx, peer_id in enumerate(all_peers, start=1):
    if idx == self_index:
        continue
    allowed.append({'name': f'node_{idx}', 'public_key': peer_id})

data['allowed_peers'] = allowed
data['min_signers'] = min_signers
data['max_signers'] = max_signers
config_path.write_text(yaml.safe_dump(data, sort_keys=False))
PY
done
echo "All configs patched."

# ------------------- Generate docker-compose.yaml ----------------------------
echo -e "\nGenerating '$DOCKER_COMPOSE_FILE' for $NUM_NODES nodes..."

cat > "$BASE_DIR/test_${NUM_NODES}_nodes/$DOCKER_COMPOSE_FILE" <<EOL
services:
EOL

for i in $(seq 1 "$NUM_NODES"); do
  node_name="node_${i}"
  node_dir="$CONFIGS_DIR/$node_name"
  host_port=$((BASE_GRPC_PORT + i - 1))

  cat >> "$BASE_DIR/test_${NUM_NODES}_nodes/$DOCKER_COMPOSE_FILE" <<EOL
  node${i}:
    image: threshold-node
    environment:
      - KEY_PASSWORD=${KEY_PASSWORD}
      - IS_TESTNET=true
      - RUST_LOG=info
    entrypoint: "/app/cli run --key-file-path /app/configs/${node_name}.json --config-file-path /app/configs/${node_name}.yaml --use-mock-oracle"
    ports:
      - "${host_port}:50051"
    volumes:
      - ./${node_name}/${node_name}.json:/app/configs/${node_name}.json
      - ./${node_name}/${node_name}.yaml:/app/configs/${node_name}.yaml
    networks:
      - vaultnet
EOL
done

cat >> "$BASE_DIR/test_${NUM_NODES}_nodes/$DOCKER_COMPOSE_FILE" <<EOL
networks:
  vaultnet:
    driver: bridge
EOL

DOCKER_COMPOSE_FILE_PATH="$BASE_DIR/test_${NUM_NODES}_nodes/$DOCKER_COMPOSE_FILE"

echo "Successfully generated '$BASE_DIR/test_${NUM_NODES}_nodes/$DOCKER_COMPOSE_FILE'."

# ------------------- Build Docker Image Once ----------------------------
echo -e "\nBuilding Docker image..."

cd "$BASE_DIR/test_${NUM_NODES}_nodes"
docker build -t threshold-node -f ../../Dockerfile ../..

# ------------------- Start Docker Compose ----------------------------
echo -e "\nStarting Docker Compose for $NUM_NODES nodes..."

docker compose -f "$DOCKER_COMPOSE_FILE" up -d

# Wait 10 seconds for dkg to complete
sleep 20s

echo "All nodes started."

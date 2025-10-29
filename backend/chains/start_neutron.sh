#!/usr/bin/env bash
set -euo pipefail

# ============================================================
# Neutron Mainnet Fork Script
# Creates a local fork using snapshot data directly (no export)
# ============================================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
BIN_DIR="$PROJECT_ROOT/bin"
NEUTRON_BIN="$BIN_DIR/neutrond-linux-amd64"

SOURCE_CHAIN_ID="neutron-1"
FORK_CHAIN_ID="${FORK_CHAIN_ID:-neutron-fork-1}"

FUND_TEST_ACCOUNT="${FUND_TEST_ACCOUNT:-1000000000000untrn}"
SELF_DELEGATION="${SELF_DELEGATION:-500000000000untrn}"
MIN_GAS="${MIN_GAS:-0untrn}"

MNEMONIC="${MNEMONIC:-banner spread envelope side kite person disagree path silver will brother under couch edit food venture squirrel civil budget number acquire point work mass}"

IMAGE_TAG="${IMAGE_TAG:-neutron-fork-img:latest}"
SNAPSHOT_VOL="${SNAPSHOT_VOL:-neutron-mainnet-snapshot}"
FORK_VOL="${FORK_VOL:-neutron-fork-1-home}"
CONTAINER_NAME="${CONTAINER_NAME:-neutron-fork-1}"

WASM_URL="https://snapshots.polkachu.com/wasm/neutron/neutron_wasmonly.tar.lz4"
SNAPSHOT_FILE="${SNAPSHOT_FILE:-}"

mkdir -p "$BIN_DIR"

# ============================================================
# Download neutrond binary
# ============================================================
if [[ ! -f "$NEUTRON_BIN" ]]; then
  echo "[prep] Downloading neutrond v8.1.1..."
  curl -L -o "$NEUTRON_BIN" "https://github.com/neutron-org/neutron/releases/download/v8.1.1/neutrond-linux-amd64"
  chmod +x "$NEUTRON_BIN"
  echo "[prep] Binary downloaded"
fi

# ============================================================
# Build Docker image
# ============================================================
echo "[docker] Building utility image..."
BUILD_DIR="$(mktemp -d)"; trap 'rm -rf "$BUILD_DIR"' EXIT
cp "$NEUTRON_BIN" "$BUILD_DIR/neutrond"
cat > "$BUILD_DIR/Dockerfile" <<'DOCKERFILE'
FROM alpine:latest
RUN apk add --no-cache bash jq curl ca-certificates lz4 wget coreutils findutils bind-tools sed gawk && update-ca-certificates
COPY neutrond /usr/local/bin/neutrond
RUN chmod +x /usr/local/bin/neutrond
DOCKERFILE
docker build -q -t "$IMAGE_TAG" "$BUILD_DIR" >/dev/null
echo "[docker] Image built: $IMAGE_TAG"

docker volume create "$SNAPSHOT_VOL" >/dev/null 2>&1
docker volume create "$FORK_VOL" >/dev/null 2>&1

# ============================================================
# STEP 1: Download snapshot
# ============================================================
echo ""
echo "========================================================"
echo "STEP 1: Downloading mainnet snapshot"
echo "========================================================"

docker run --rm \
  -v "$SNAPSHOT_VOL":/root/.neutrond \
  --entrypoint bash \
  "$IMAGE_TAG" -c '
set -e
HOME_DIR=/root/.neutrond
DATA_DIR="$HOME_DIR/data"
WASM_URL="'"$WASM_URL"'"
SNAPSHOT_FILE_OVERRIDE="'"$SNAPSHOT_FILE"'"

if [[ -d "$DATA_DIR" ]] && [[ -f "$DATA_DIR/priv_validator_state.json" ]]; then
  echo "[snapshot] Found existing snapshot data"
  exit 0
fi

mkdir -p "$HOME_DIR"

if [[ -n "$SNAPSHOT_FILE_OVERRIDE" ]]; then
  SNAPSHOT_FILE="$SNAPSHOT_FILE_OVERRIDE"
  SNAPSHOT_URL="https://snapshots.polkachu.com/snapshots/neutron/$SNAPSHOT_FILE"
  echo "[snapshot] Using: $SNAPSHOT_FILE"
else
  echo "[snapshot] Detecting latest snapshot..."
  SNAPSHOT_PAGE=$(curl -sL "https://www.polkachu.com/tendermint_snapshots/neutron" || echo "")
  SNAPSHOT_FILE=$(echo "$SNAPSHOT_PAGE" | grep -oE "neutron_[0-9]+\.tar\.lz4" | sort -u | tail -1)
  
  if [[ -z "$SNAPSHOT_FILE" ]]; then
    echo "[snapshot] ERROR: Could not detect snapshot"
    echo "[snapshot] Please set: SNAPSHOT_FILE=neutron_XXXXXXXX.tar.lz4"
    exit 1
  fi
  
  SNAPSHOT_FILE=$(echo "$SNAPSHOT_FILE" | sed "s/<[^>]*>//g" | tr -cd "a-zA-Z0-9._-")
  SNAPSHOT_URL="https://snapshots.polkachu.com/snapshots/neutron/$SNAPSHOT_FILE"
  echo "[snapshot] Selected: $SNAPSHOT_FILE"
fi

echo "[snapshot] Downloading (10-30 minutes)..."
if ! curl -fSL --progress-bar "$SNAPSHOT_URL" | lz4 -d | tar -x -C "$HOME_DIR"; then
  echo "[snapshot] ERROR: Download failed"
  exit 1
fi

echo "[snapshot] Downloading wasm..."
curl -fSL "$WASM_URL" | lz4 -d | tar -x -C "$HOME_DIR" 2>/dev/null || echo "[snapshot] Wasm download failed (ok)"

echo "[snapshot] Snapshot ready"
'

# ============================================================
# STEP 2: Download and prepare genesis
# ============================================================
echo ""
echo "========================================================"
echo "STEP 2: Preparing genesis file"
echo "========================================================"

GENESIS_SOURCES=(
  "https://snapshots.polkachu.com/genesis/neutron/genesis.json"
  "https://raw.githubusercontent.com/neutron-org/mainnet-assets/main/genesis.json"
  "https://neutron-1.rpc.p2p.world/genesis"
)

echo "[genesis] Downloading mainnet genesis..."
for URL in "${GENESIS_SOURCES[@]}"; do
  echo "[genesis] Trying: $URL"
  if curl -fSL --max-time 60 "$URL" -o "$SCRIPT_DIR/mainnet_genesis.json" 2>/dev/null; then
    if jq empty "$SCRIPT_DIR/mainnet_genesis.json" 2>/dev/null; then
      echo "[genesis] Downloaded successfully"
      break
    fi
    rm -f "$SCRIPT_DIR/mainnet_genesis.json"
  fi
done

if [[ ! -f "$SCRIPT_DIR/mainnet_genesis.json" ]]; then
  echo "[genesis] ERROR: Could not download genesis.json"
  exit 1
fi

echo "[genesis] Modifying for fork (chain-id: $FORK_CHAIN_ID)..."
jq --arg chain_id "$FORK_CHAIN_ID" '.chain_id = $chain_id' "$SCRIPT_DIR/mainnet_genesis.json" > "$SCRIPT_DIR/fork_genesis.json"

# ============================================================
# STEP 3: Create fork from snapshot
# ============================================================
echo ""
echo "========================================================"
echo "STEP 3: Creating fork from snapshot data"
echo "========================================================"

echo "[fork] Copying snapshot data to fork volume..."
docker run --rm \
  -v "$SNAPSHOT_VOL":/source \
  -v "$FORK_VOL":/dest \
  --entrypoint bash \
  "$IMAGE_TAG" -c '
echo "Copying data..."
rm -rf /dest/data /dest/wasm 2>/dev/null || true
cp -r /source/data /dest/ 2>/dev/null && echo "Data copied" || echo "No data dir"
cp -r /source/wasm /dest/ 2>/dev/null && echo "Wasm copied" || echo "No wasm dir"
'

echo "[fork] Initializing fork configuration..."
docker run --rm \
  -v "$FORK_VOL":/root/.neutronfork \
  -v "$SCRIPT_DIR":/work \
  --entrypoint bash \
  "$IMAGE_TAG" -c '
NEW_HOME=/root/.neutronfork
echo "Initializing fork..."
neutrond init local --chain-id '"$FORK_CHAIN_ID"' --home "$NEW_HOME" --overwrite >/dev/null 2>&1
cp /work/fork_genesis.json "$NEW_HOME/config/genesis.json"
echo "Fork initialized with mainnet genesis"
'

# ============================================================
# STEP 4: Setup deployer account and validator
# ============================================================
echo ""
echo "========================================================"
echo "STEP 4: Setting up deployer and validator"
echo "========================================================"

# First, get the deployer address
DEPLOYER_ADDR=$(docker run --rm \
  -e MNEMONIC="$MNEMONIC" \
  --entrypoint bash \
  "$IMAGE_TAG" -c '
echo "$MNEMONIC" | neutrond keys add deployer --recover --keyring-backend=test --home /tmp/temp_home >/dev/null 2>&1
neutrond keys show deployer -a --keyring-backend=test --home /tmp/temp_home 2>/dev/null
')

echo "[keys] Deployer address: $DEPLOYER_ADDR"

# Modify genesis to add/update the deployer balance
echo "[genesis] Adding deployer funds to genesis..."
docker run --rm \
  -v "$FORK_VOL":/root/.neutronfork \
  --entrypoint bash \
  "$IMAGE_TAG" -c '
NEW_HOME=/root/.neutronfork
GENESIS="$NEW_HOME/config/genesis.json"
DEPLOYER_ADDR="'"$DEPLOYER_ADDR"'"
FUND_AMOUNT="'"$FUND_TEST_ACCOUNT"'"

# Extract amount and denom
AMOUNT="${FUND_AMOUNT//[^0-9]/}"
DENOM="${FUND_AMOUNT//[0-9]/}"

echo "Setting balance for $DEPLOYER_ADDR to $AMOUNT$DENOM"

# Create a temporary jq script for complex operation
cat > /tmp/update_balance.jq <<EOF
# Find and update or add the account balance
.app_state.bank.balances = (
  .app_state.bank.balances | 
  map(if .address == "$DEPLOYER_ADDR" then
    # Update existing account - replace untrn balance
    .coins = ([.coins[] | select(.denom != "$DENOM")] + [{"denom": "$DENOM", "amount": "$AMOUNT"}])
  else . end)
) |
# If account was not found, add it
if ([.app_state.bank.balances[] | select(.address == "$DEPLOYER_ADDR")] | length) == 0 then
  .app_state.bank.balances += [{
    "address": "$DEPLOYER_ADDR",
    "coins": [{"denom": "$DENOM", "amount": "$AMOUNT"}]
  }]
else . end |
# Update total supply - add the difference
.app_state.bank.supply = (
  .app_state.bank.supply |
  map(if .denom == "$DENOM" then
    .amount = (((.amount | tonumber) + ($AMOUNT | tonumber)) | tostring)
  else . end)
)
EOF

# Apply the jq transformation
jq -f /tmp/update_balance.jq "$GENESIS" > "$GENESIS.tmp"

# Validate the result
if jq empty "$GENESIS.tmp" 2>/dev/null; then
  mv "$GENESIS.tmp" "$GENESIS"
  echo "Genesis updated successfully"
  
  # Verify the balance was set
  BALANCE=$(jq -r ".app_state.bank.balances[] | select(.address == \"$DEPLOYER_ADDR\") | .coins[] | select(.denom == \"$DENOM\") | .amount" "$GENESIS")
  echo "Verified balance in genesis: $BALANCE$DENOM"
else
  echo "ERROR: Genesis update produced invalid JSON"
  rm "$GENESIS.tmp"
  exit 1
fi
'

# Now setup the validator
docker run --rm \
  -e MNEMONIC="$MNEMONIC" \
  -v "$FORK_VOL":/root/.neutronfork \
  --entrypoint bash \
  "$IMAGE_TAG" -c '
NEW_HOME=/root/.neutronfork

echo "[keys] Importing deployer key..."
echo "$MNEMONIC" | neutrond keys add deployer --recover --keyring-backend=test --home "$NEW_HOME" >/dev/null 2>&1 || true

echo "[validator] Creating validator..."
neutrond gentx deployer '"$SELF_DELEGATION"' \
  --chain-id '"$FORK_CHAIN_ID"' \
  --keyring-backend=test \
  --home "$NEW_HOME" \
  --moniker="fork-validator" \
  --commission-rate=0.1 \
  --commission-max-rate=0.2 \
  --commission-max-change-rate=0.01 \
  --min-self-delegation=1 \
  >/dev/null 2>&1

neutrond collect-gentxs --home "$NEW_HOME" >/dev/null 2>&1
echo "[validator] Validator configured"
'

# ============================================================
# STEP 5: Configure chain settings
# ============================================================
echo ""
echo "========================================================"
echo "STEP 5: Configuring chain"
echo "========================================================"

docker run --rm \
  -v "$FORK_VOL":/root/.neutronfork \
  --entrypoint bash \
  "$IMAGE_TAG" -c '
NEW_HOME=/root/.neutronfork
APP="$NEW_HOME/config/app.toml"
CFG="$NEW_HOME/config/config.toml"

echo "[config] Enabling APIs..."
sed -i "s/enable = false/enable = true/g" "$APP"
sed -i "s/swagger = false/swagger = true/g" "$APP"

echo "[config] Opening interfaces..."
sed -i "s/address = \"tcp:\/\/localhost:1317\"/address = \"tcp:\/\/0.0.0.0:1317\"/" "$APP"
sed -i "s/address = \"localhost:9090\"/address = \"0.0.0.0:9090\"/" "$APP"
sed -i "s/address = \"localhost:9091\"/address = \"0.0.0.0:9091\"/" "$APP"
sed -i "s/laddr = \"tcp:\/\/127.0.0.1:26657\"/laddr = \"tcp:\/\/0.0.0.0:26657\"/" "$CFG"

echo "[config] Isolating from mainnet..."
sed -i "s/^persistent_peers = .*/persistent_peers = \"\"/" "$CFG"
sed -i "s/^seeds = .*/seeds = \"\"/" "$CFG"
sed -i "s/^pex = true/pex = false/" "$CFG"

echo "[config] Faster blocks..."
sed -i "s/^timeout_commit = .*/timeout_commit = \"1s\"/" "$CFG"
sed -i "s/^timeout_propose = .*/timeout_propose = \"1s\"/" "$CFG"

echo "[config] Done"
'

# ============================================================
# STEP 6: Start the chain
# ============================================================
echo ""
echo "========================================================"
echo "STEP 6: Starting fork"
echo "========================================================"

docker rm -f "$CONTAINER_NAME" >/dev/null 2>&1 || true

docker run -d \
  --name "$CONTAINER_NAME" \
  -v "$FORK_VOL":/root/.neutronfork \
  -p 26657:26657 -p 1317:1317 -p 9090:9090 \
  "$IMAGE_TAG" \
  neutrond start \
    --home /root/.neutronfork \
    --minimum-gas-prices="$MIN_GAS" \
    --rpc.laddr=tcp://0.0.0.0:26657 \
    --grpc.address=0.0.0.0:9090 \
    --api.address=tcp://0.0.0.0:1317 \
    --pruning=nothing \
    --log_level=info

echo "[wait] Waiting for chain to start..."
for i in {1..120}; do
  if curl -s http://localhost:26657/status >/dev/null 2>&1; then 
    break
  fi
  if ((i % 10 == 0)); then
    echo -n "."
  fi
  sleep 1
done
echo ""

if ! curl -s http://localhost:26657/status >/dev/null 2>&1; then
  echo ""
  echo "ERROR: Chain failed to start"
  echo "Logs:"
  docker logs --tail=100 "$CONTAINER_NAME"
  exit 1
fi

HEIGHT="$(curl -s http://localhost:26657/status | jq -r ".result.sync_info.latest_block_height")"
ADDR="$(docker run --rm -v "$FORK_VOL":/root/.neutronfork "$IMAGE_TAG" neutrond keys show deployer -a --keyring-backend=test --home /root/.neutronfork 2>/dev/null)"

echo ""
echo "========================================================"
echo "SUCCESS: Neutron Fork Running!"
echo "========================================================"
echo ""
echo "Chain ID:       $FORK_CHAIN_ID"
echo "Block Height:   $HEIGHT"
echo ""
echo "ENDPOINTS:"
echo "  RPC:          http://localhost:26657"
echo "  REST:         http://localhost:1317"
echo "  gRPC:         http://localhost:9090"
echo ""
echo "DEPLOYER:"
echo "  Address:      $ADDR"
echo "  Balance:      $FUND_TEST_ACCOUNT"
echo "  Validator:    YES (stake: $SELF_DELEGATION)"
echo ""
echo "COMMANDS:"
echo "  Logs:         docker logs -f $CONTAINER_NAME"
echo "  Stop:         docker stop $CONTAINER_NAME"
echo "  Start:        docker start $CONTAINER_NAME"
echo "  Shell:        docker exec -it $CONTAINER_NAME sh"
echo ""
echo "RESET:"
echo "  docker rm -f $CONTAINER_NAME"
echo "  docker volume rm $SNAPSHOT_VOL $FORK_VOL"
echo ""
echo "========================================================"

#!/usr/bin/env bash
set -euo pipefail

# --- Config ---
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
NEUTRON_BIN="$PROJECT_ROOT/bin/neutrond-linux-amd64"
CHAIN_ID="neutron-1"
IMAGE_TAG="neutron-fork-img:untrn"

# Fund test account with this amount (set to empty string to skip funding)
FUND_TEST_ACCOUNT="${FUND_TEST_ACCOUNT:-1000000000000untrn}"  # 1M NTRN

# Polkachu provides reliable Neutron snapshots
SNAPSHOT_BASE_URL="https://snapshots.polkachu.com/snapshots/neutron"

mkdir -p "$PROJECT_ROOT/bin"

# --- Get binary if missing ---
if [[ ! -f "$NEUTRON_BIN" ]]; then
  echo "Neutron binary not found at $NEUTRON_BIN"
  read -p "Download neutrond v8.1.1? [Y/n] " -n 1 -r; echo
  if [[ ! $REPLY =~ ^[Yy]$ ]] && [[ -n ${REPLY:-} ]]; then
    echo "Exiting."; exit 1
  fi
  echo "Downloading neutrond..."
  wget -O "$NEUTRON_BIN" https://github.com/neutron-org/neutron/releases/download/v8.1.1/neutrond-linux-amd64
  chmod +x "$NEUTRON_BIN"
fi

# --- Build image with snapshot support ---
BUILD_DIR="$(mktemp -d)"; trap 'rm -rf "$BUILD_DIR"' EXIT
cp "$NEUTRON_BIN" "$BUILD_DIR/neutrond"

cat > "$BUILD_DIR/Dockerfile" <<'DOCKER'
FROM alpine:latest
RUN apk add --no-cache bash jq curl ca-certificates lz4 wget && update-ca-certificates
COPY neutrond /usr/local/bin/neutrond
RUN chmod +x /usr/local/bin/neutrond
COPY entrypoint.sh /entrypoint.sh
EXPOSE 26657 1317 9090
ENTRYPOINT ["/bin/bash","/entrypoint.sh"]
DOCKER

cat > "$BUILD_DIR/entrypoint.sh" <<'ENTRY'
#!/usr/bin/env bash
set -euo pipefail

CHAIN_ID="neutron-1"
HOME_DIR="/root/.neutrond"
CONFIG_DIR="$HOME_DIR/config"
DATA_DIR="$HOME_DIR/data"

# Fix config issues BEFORE running any neutrond commands
if [[ -f "$CONFIG_DIR/config.toml" ]]; then
  echo "[entrypoint] Fixing config compatibility issues..."
  # Fix skip_timeout_commit parsing error (snapshot config may have wrong type)
  sed -i 's/^skip_timeout_commit = .*/skip_timeout_commit = false/' "$CONFIG_DIR/config.toml" 2>/dev/null || true
fi

echo "[entrypoint] neutrond version:"
neutrond version || { echo "[fatal] neutrond not runnable"; exit 1; }

# Create deployer key for local testing
MNEMONIC="banner spread envelope side kite person disagree path silver will brother under couch edit food venture squirrel civil budget number acquire point work mass"
echo "$MNEMONIC" | neutrond keys add deployer --recover --keyring-backend=test 2>/dev/null || true
DEPLOYER="$(neutrond keys show deployer -a --keyring-backend=test)"
echo "[entrypoint] Local test account: $DEPLOYER"

# Check if we already have data (check for data directory and some blocks)
if [[ -d "$DATA_DIR" ]] && [[ -f "$DATA_DIR/priv_validator_state.json" ]] && [[ -d "$DATA_DIR/application.db" || -d "$DATA_DIR/blockstore.db" ]]; then
  echo "[entrypoint] ✓ Found existing blockchain data, skipping snapshot download"
  BLOCK_COUNT=$(find "$DATA_DIR" -type f 2>/dev/null | wc -l)
  echo "[entrypoint]   Data directory contains $BLOCK_COUNT files"
else
  echo "[entrypoint] Downloading mainnet snapshot from Polkachu..."
  
  mkdir -p "$HOME_DIR"
  cd "$HOME_DIR"
  
  # Fetch the snapshot page to get the actual filename
  echo "[entrypoint] Fetching latest snapshot info..."
  SNAPSHOT_PAGE=$(curl -s "https://www.polkachu.com/tendermint_snapshots/neutron" || echo "")
  
  if [[ -z "$SNAPSHOT_PAGE" ]]; then
    echo "[error] Failed to fetch snapshot info from Polkachu"
    exit 1
  fi
  
  # Extract just the filename by looking for the pattern in the URL
  # The page contains: https://snapshots.polkachu.com/snapshots/neutron/neutron_XXXXXX.tar.lz4
  SNAPSHOT_FILE=$(echo "$SNAPSHOT_PAGE" | grep -o 'snapshots\.polkachu\.com/snapshots/neutron/neutron_[0-9]*\.tar\.lz4' | sed 's|.*/||' | head -1)
  
  # Fallback: look for just the filename pattern alone
  if [[ -z "$SNAPSHOT_FILE" ]]; then
    echo "[warning] Method 1 failed, trying method 2..."
    SNAPSHOT_FILE=$(echo "$SNAPSHOT_PAGE" | sed 's/</\n</g' | grep 'neutron_[0-9]*\.tar\.lz4' | sed 's/.*neutron_/neutron_/' | sed 's/\.tar\.lz4.*/\.tar\.lz4/' | head -1)
  fi
  
  # Another fallback: extract from wget command
  if [[ -z "$SNAPSHOT_FILE" ]]; then
    echo "[warning] Method 2 failed, trying method 3..."
    SNAPSHOT_FILE=$(echo "$SNAPSHOT_PAGE" | grep 'wget -O' | grep -o 'neutron_[0-9]*\.tar\.lz4' | head -1)
  fi
  
  if [[ -z "$SNAPSHOT_FILE" ]]; then
    echo "[error] Could not determine snapshot filename"
    echo "[info] Visit https://www.polkachu.com/tendermint_snapshots/neutron for manual download"
    exit 1
  fi
  
  # Clean up any remaining HTML artifacts
  SNAPSHOT_FILE=$(echo "$SNAPSHOT_FILE" | sed 's/<[^>]*>//g' | sed 's/[^a-zA-Z0-9._-]//g')
  
  SNAPSHOT_URL="https://snapshots.polkachu.com/snapshots/neutron/$SNAPSHOT_FILE"
  echo "[entrypoint] Found snapshot: $SNAPSHOT_FILE"
  echo "[entrypoint] Downloading from: $SNAPSHOT_URL"
  echo "[entrypoint] This will take 10-30 minutes depending on connection speed..."
  
  # Use curl with streaming to save disk space - show progress
  if ! curl -L --progress-bar "$SNAPSHOT_URL" | lz4 -c -d - | tar -x -C "$HOME_DIR"; then
    echo "[error] Failed to download and extract snapshot"
    echo "[error] URL tried: $SNAPSHOT_URL"
    exit 1
  fi
  
  # Also download the wasm folder (Neutron requires this)
  echo "[entrypoint] Downloading wasm data..."
  
  # Construct wasm filename (usually neutron_wasmonly.tar.lz4)
  WASM_URL="https://snapshots.polkachu.com/wasm/neutron/neutron_wasmonly.tar.lz4"
  
  if curl -f -L --progress-bar "$WASM_URL" | lz4 -c -d - | tar -x -C "$HOME_DIR" 2>/dev/null; then
    echo "[entrypoint] Wasm data downloaded successfully"
  else
    echo "[warning] Failed to download wasm folder, but continuing..."
  fi
  
  echo "[entrypoint] Snapshot extracted successfully"
fi

# Ensure config exists
if [[ ! -f "$CONFIG_DIR/config.toml" ]]; then
  echo "[entrypoint] Initializing config..."
  neutrond init local --chain-id="$CHAIN_ID" --overwrite
fi

# Download genesis.json if missing (snapshot may not include it)
if [[ ! -f "$CONFIG_DIR/genesis.json" ]]; then
  echo "[entrypoint] Downloading genesis.json..."
  wget -q -O "$CONFIG_DIR/genesis.json" "https://snapshots.polkachu.com/genesis/neutron/genesis.json" || {
    curl -s -o "$CONFIG_DIR/genesis.json" "https://snapshots.polkachu.com/genesis/neutron/genesis.json" || {
      echo "[error] Failed to download genesis.json"
      exit 1
    }
  }
  echo "[entrypoint] Genesis file downloaded"
fi

# --- Mainnet Fork Modifications ---

echo "[entrypoint] Configuring for local fork..."

# 1. Enable API and Swagger
sed -i 's/enable = false/enable = true/g' "$CONFIG_DIR/app.toml"
sed -i 's/swagger = false/swagger = true/' "$CONFIG_DIR/app.toml"

# 2. Open up addresses for external access
sed -i 's/address = "tcp:\/\/localhost:1317"/address = "tcp:\/\/0.0.0.0:1317"/' "$CONFIG_DIR/app.toml"
sed -i 's/address = "localhost:9090"/address = "0.0.0.0:9090"/' "$CONFIG_DIR/app.toml"
sed -i 's/address = "localhost:9091"/address = "0.0.0.0:9091"/' "$CONFIG_DIR/app.toml"

# 3. Configure Tendermint for local access
sed -i 's/laddr = "tcp:\/\/127.0.0.1:26657"/laddr = "tcp:\/\/0.0.0.0:26657"/' "$CONFIG_DIR/config.toml"

# 4. Disable external peers (run in isolation)
sed -i 's/^persistent_peers = .*/persistent_peers = ""/' "$CONFIG_DIR/config.toml"
sed -i 's/^seeds = .*/seeds = ""/' "$CONFIG_DIR/config.toml"

# 5. Speed up block times for testing (optional)
sed -i 's/^timeout_commit = .*/timeout_commit = "1s"/' "$CONFIG_DIR/config.toml"
sed -i 's/^timeout_propose = .*/timeout_propose = "1s"/' "$CONFIG_DIR/config.toml"

# 6. Disable PEX and other p2p features
sed -i 's/^pex = true/pex = false/' "$CONFIG_DIR/config.toml"

echo "[entrypoint] Starting forked node..."

# Fund test account if requested (only works on fresh genesis, before first start)
FUND_AMOUNT="${FUND_TEST_ACCOUNT:-}"
FUNDED_MARKER="$HOME_DIR/.funded"

if [[ -n "$FUND_AMOUNT" ]] && [[ ! -f "$FUNDED_MARKER" ]]; then
  # Only try to fund if this looks like a fresh start (no blocks processed yet)
  if [[ ! -f "$DATA_DIR/priv_validator_state.json" ]] || ! grep -q '"height": *"[1-9]' "$DATA_DIR/priv_validator_state.json" 2>/dev/null; then
    echo "[entrypoint] Funding test account with $FUND_AMOUNT in genesis..."
    
    # Parse the fund amount
    AMOUNT=$(echo "$FUND_AMOUNT" | sed 's/[^0-9]//g')
    DENOM=$(echo "$FUND_AMOUNT" | sed 's/[0-9]//g')
    
    # Modify genesis.json directly
    if jq --arg addr "$DEPLOYER" --arg amount "$AMOUNT" --arg denom "$DENOM" \
      '(.app_state.bank.balances // []) |= . + [{"address": $addr, "coins": [{"denom": $denom, "amount": $amount}]}] |
       .app_state.bank.supply |= (map(if .denom == $denom then .amount = ((.amount | tonumber) + ($amount | tonumber) | tostring) else . end))' \
      "$CONFIG_DIR/genesis.json" > "$CONFIG_DIR/genesis.json.tmp" 2>/dev/null; then
      
      mv "$CONFIG_DIR/genesis.json.tmp" "$CONFIG_DIR/genesis.json"
      touch "$FUNDED_MARKER"
      echo "[entrypoint] ✓ Test account funded with $FUND_AMOUNT"
    else
      echo "[entrypoint] ⚠️  Could not modify genesis, account not funded"
      rm -f "$CONFIG_DIR/genesis.json.tmp"
    fi
  else
    echo "[entrypoint] Chain already started, cannot fund via genesis modification"
    touch "$FUNDED_MARKER"
  fi
fi

echo "[entrypoint] NOTE: Using mainnet state"
if [[ -f "$FUNDED_MARKER" ]]; then
  echo "[entrypoint]       Test account has been funded with $FUND_AMOUNT"
elif [[ -z "$FUND_AMOUNT" ]]; then
  echo "[entrypoint]       Test account ($DEPLOYER) has no funds"
  echo "[entrypoint]       Use an existing mainnet account with funds for testing"
fi

# Start the node with low gas prices for easier testing
exec neutrond start \
  --minimum-gas-prices=0.001untrn \
  --rpc.laddr=tcp://0.0.0.0:26657 \
  --grpc.address=0.0.0.0:9090 \
  --api.address=tcp://0.0.0.0:1317 \
  --pruning=nothing \
  --log_level=info
ENTRY
chmod +x "$BUILD_DIR/entrypoint.sh"

echo "Building $IMAGE_TAG ..."
docker build -t "$IMAGE_TAG" "$BUILD_DIR"

# --- Run container ---
echo "Starting Neutron mainnet fork..."
docker rm -f neutron-fork >/dev/null 2>&1 || true

# Create a named volume to persist the snapshot across restarts
docker volume create neutron-fork-data >/dev/null 2>&1 || true

docker run -d \
  --name neutron-fork \
  -e CHAIN_ID="$CHAIN_ID" \
  -e FUND_TEST_ACCOUNT="$FUND_TEST_ACCOUNT" \
  -v neutron-fork-data:/root/.neutrond \
  -p 26657:26657 \
  -p 1317:1317 \
  -p 9090:9090 \
  "$IMAGE_TAG"

# --- Wait until ready ---
echo ""
echo "Waiting for Neutron fork to start..."
echo "(First run will download ~5-10GB snapshot - this takes 10-30 minutes)"
echo ""

for i in {1..600}; do
  if curl -s http://localhost:26657/status >/dev/null 2>&1; then 
    echo ""
    break
  fi
  if ((i % 10 == 0)); then
    echo -n "."
  fi
  sleep 2
done

if ! curl -s http://localhost:26657/status >/dev/null 2>&1; then
  echo ""
  echo "Node failed to start. Recent logs:"
  docker logs --tail=100 neutron-fork
  exit 1
fi

# Get current block height
BLOCK_HEIGHT=$(curl -s http://localhost:26657/status | jq -r '.result.sync_info.latest_block_height')

echo "✓ Neutron mainnet fork is running!"
echo ""
echo "  RPC:         http://localhost:26657"
echo "  REST API:    http://localhost:1317"
echo "  gRPC:        http://localhost:9090"
echo "  Chain ID:    $CHAIN_ID"
echo "  Block Height: $BLOCK_HEIGHT (mainnet state)"
echo ""
echo "  Test Account: $(docker exec neutron-fork neutrond keys show deployer -a --keyring-backend=test 2>/dev/null)"
if [[ -n "$FUND_TEST_ACCOUNT" ]]; then
  echo "  Funded With:  $FUND_TEST_ACCOUNT"
fi
echo ""
echo "⚠️  MAINNET FORK NOTES:"
echo "    - Uses real mainnet state (all contracts, accounts preserved)"
if [[ -n "$FUND_TEST_ACCOUNT" ]]; then
  echo "    - Test account has been funded with $FUND_TEST_ACCOUNT"
else
  echo "    - Test account has NO funds (use mainnet account or set FUND_TEST_ACCOUNT)"
fi
echo ""
echo "💾 DATA PERSISTENCE:"
echo "    Snapshot data is cached in Docker volume 'neutron-fork-data'"
echo "    Subsequent restarts will be instant (no re-download)"
echo ""
echo "🔧 USAGE:"
echo "    To disable funding: FUND_TEST_ACCOUNT=\"\" ./start_neutron_fork.sh"
echo "    To change amount:   FUND_TEST_ACCOUNT=\"5000000000000untrn\" ./start_neutron_fork.sh"
echo ""
echo "To view logs:       docker logs -f neutron-fork"
echo "To stop:            docker stop neutron-fork"
echo "To restart:         docker start neutron-fork"
echo "To remove & reset:  docker rm -f neutron-fork && docker volume rm neutron-fork-data"
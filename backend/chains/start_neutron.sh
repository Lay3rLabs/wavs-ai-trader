#!/usr/bin/env bash
set -euo pipefail

# --- Config ---
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
NEUTRON_BIN="$PROJECT_ROOT/bin/neutrond-linux-amd64"
CHAIN_ID="pion-1"
IMAGE_TAG="neutron-local-img:untrn"

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

# --- Build a tiny image with the binary + entrypoint ---
BUILD_DIR="$(mktemp -d)"; trap 'rm -rf "$BUILD_DIR"' EXIT
cp "$NEUTRON_BIN" "$BUILD_DIR/neutrond"

cat > "$BUILD_DIR/Dockerfile" <<'DOCKER'
FROM alpine:latest
RUN apk add --no-cache bash jq curl ca-certificates && update-ca-certificates
COPY neutrond /usr/local/bin/neutrond
RUN chmod +x /usr/local/bin/neutrond
COPY entrypoint.sh /entrypoint.sh
EXPOSE 26657 1317 9090
ENTRYPOINT ["/bin/bash","/entrypoint.sh"]
DOCKER

cat > "$BUILD_DIR/entrypoint.sh" <<'ENTRY'
#!/usr/bin/env bash
set -euo pipefail

CHAIN_ID="pion-1"
HOME_DIR="/root/.neutrond"
GEN="$HOME_DIR/config/genesis.json"

echo "[entrypoint] neutrond version:"
neutrond version || { echo "[fatal] neutrond not runnable"; exit 1; }

echo "[entrypoint] init chain: $CHAIN_ID"
neutrond init local --chain-id="$CHAIN_ID"

MNEMONIC="banner spread envelope side kite person disagree path silver will brother under couch edit food venture squirrel civil budget number acquire point work mass"
echo "$MNEMONIC" | neutrond keys add deployer --recover --keyring-backend=test
DEPLOYER="$(neutrond keys show deployer -a --keyring-backend=test)"
echo "[entrypoint] deployer: $DEPLOYER"

# Fund ONLY in untrn
neutrond add-genesis-account "$DEPLOYER" 1000000000000untrn

# JQ filter:
# - make untrn first-class
# - seed dynamicfees
# - DISABLE feemarket (so node flag/globalfee control min gas)
# - set globalfee min gas price to 0.025 untrn (if module exists)
cat > /tmp/genesis_untrn.jq <<'JQ'
  .app_state.staking.params.bond_denom = "untrn" |
  .app_state.mint.params.mint_denom   = "untrn" |
  .app_state.crisis.constant_fee.denom = "untrn" |
  (if .app_state.gov?.deposit_params? then
     .app_state.gov.deposit_params.min_deposit =
       [ { "denom":"untrn", "amount": ( .app_state.gov.deposit_params.min_deposit[0].amount // "10000000" ) } ]
   else . end) |
  ( .app_state.bank.denom_metadata |=
      ( ( . // [] ) + [{
          "base":"untrn","display":"ntrn","name":"Neutron","symbol":"NTRN",
          "denom_units":[
            {"denom":"untrn","exponent":0},
            {"denom":"ntrn","exponent":6}
          ]
        }] | unique_by(.base)
      )
  ) |
  # Dynamic Fees (denom resolver) — ensure untrn is recognized
  ( .app_state.dynamicfees = ( .app_state.dynamicfees // { "params": { "ntrn_prices": [] } } ) ) |
  ( .app_state.dynamicfees.params.ntrn_prices = [ { "denom":"untrn", "amount":"1.000000000000000000" } ] ) |
  # Feemarket — disable for deterministic local dev
  ( if .app_state.feemarket? and .app_state.feemarket.params? then
      .app_state.feemarket.params.enabled = false
    else . end ) |
  # GlobalFee (if present) — set minimum gas price in untrn
  ( if .app_state.globalfee? then
      .app_state.globalfee.params.minimum_gas_prices =
        [ { "denom":"untrn", "amount":"0.025000000000000000" } ]
    else . end )
JQ

echo "[entrypoint] patching genesis..."
jq -f /tmp/genesis_untrn.jq "$GEN" > "$GEN.tmp" && mv "$GEN.tmp" "$GEN"

echo "[entrypoint] gentx in untrn"
neutrond gentx deployer 100000000untrn --chain-id="$CHAIN_ID" --keyring-backend=test
neutrond collect-gentxs

echo "[entrypoint] enable API + swagger"
sed -i 's/enable = false/enable = true/' "$HOME_DIR/config/app.toml"
sed -i 's/swagger = false/swagger = true/' "$HOME_DIR/config/app.toml"

echo "[entrypoint] starting node..."
exec neutrond start --minimum-gas-prices=0.025untrn --rpc.laddr=tcp://0.0.0.0:26657
ENTRY
chmod +x "$BUILD_DIR/entrypoint.sh"

echo "Building $IMAGE_TAG ..."
docker build -t "$IMAGE_TAG" "$BUILD_DIR" >/dev/null

# --- Run container ---
echo "Starting Neutron local node..."
docker rm -f neutron-local >/dev/null 2>&1 || true
docker run -d \
  --name neutron-local \
  -e CHAIN_ID="$CHAIN_ID" \
  -p 26657:26657 -p 1317:1317 -p 9090:9090 \
  "$IMAGE_TAG" >/dev/null

# --- Wait until ready ---
echo "Waiting for Neutron..."
for i in {1..60}; do
  if curl -s http://localhost:26657/status >/dev/null 2>&1; then break; fi
  sleep 1
done

if ! curl -s http://localhost:26657/status >/dev/null 2>&1; then
  echo "Node failed to start. Recent logs:"; docker logs --tail=200 neutron-local; exit 1
fi

echo "✓ Neutron is running"
echo "  RPC:  http://localhost:26657"
echo "  REST: http://localhost:1317"
echo "  gRPC: http://localhost:9090"

DEPLOYER="$(docker exec neutron-local neutrond keys show deployer -a --keyring-backend=test)"
echo "  Deployer: $DEPLOYER"

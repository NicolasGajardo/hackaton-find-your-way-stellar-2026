#!/usr/bin/env bash
# Deploys FondexVault to testnet, accepts the DeFindex Blend USDC strategy
# and makes Fondex's initial deposit. Does not commit the README.
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
# shellcheck source=lib/testnet.sh
source "${SCRIPT_DIR}/lib/testnet.sh"

usage() {
  cat <<EOF
Usage: ./scripts/deploy-testnet.sh [flags]

  --check-only    Only preflight checks and a clean git tree (steps 1 and 2)
  --allow-dirty   Allow deploying with uncommitted changes
  --update-app    Ask before writing ${APP_ENV_REL}
  -h, --help      Show this help

Variables: scripts/testnet.env (can be overridden with VAR=…).
EOF
}

ALLOW_DIRTY=0
CHECK_ONLY=0
UPDATE_APP=0
while [[ $# -gt 0 ]]; do
  case "$1" in
    --allow-dirty) ALLOW_DIRTY=1 ;;
    --check-only) CHECK_ONLY=1 ;;
    --update-app) UPDATE_APP=1 ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      echo "unknown flag: $1" >&2
      usage
      exit 1
      ;;
  esac
  shift
done

cd "$ROOT"
load_testnet_env

CURRENT_STEP=0
CURRENT_STEP_NAME=""
VAULT_ID=""

on_err() {
  local code=$?
  if [[ -n "${VAULT_ID:-}" ]]; then
    echo "ERROR: the contract is already deployed at ${VAULT_ID}." >&2
    echo "Failed at step ${CURRENT_STEP} (${CURRENT_STEP_NAME}). You can continue manually from there." >&2
  fi
  exit "$code"
}
trap on_err ERR

step() {
  CURRENT_STEP="$1"
  CURRENT_STEP_NAME="$2"
  echo
  echo "==> Step ${CURRENT_STEP}: ${CURRENT_STEP_NAME}"
}

preflight() {
  step 1 "preflight checks"
  require_cmd stellar jq git make
  echo "    stellar $(stellar --version | head -1)"
  echo "    jq $(jq --version)"
  require_identity "$STELLAR_ACCOUNT"
  ADMIN_ADDR=$(identity_address "$STELLAR_ACCOUNT")
  echo "    identity ${STELLAR_ACCOUNT} = ${ADMIN_ADDR}"

  local rc bal=0
  set +e
  bal=$(usdc_balance "$ADMIN_ADDR")
  rc=$?
  set -e
  if [[ $rc -eq 1 ]]; then
    die "could not read the Blend USDC balance"
  fi
  if [[ $rc -eq 2 || "$bal" -lt "$INITIAL_DEPOSIT" ]]; then
    print_blend_usdc_help "$ADMIN_ADDR"
    if [[ $rc -eq 2 ]]; then
      echo "Detail: the SAC reported a missing trustline (balance treated as 0)." >&2
    else
      echo "Current balance: $(fmt_usdc "$bal") (${bal} units)" >&2
    fi
    exit 1
  fi
  echo "    Blend USDC: $(fmt_usdc "$bal") (>= $(fmt_usdc "$INITIAL_DEPOSIT"))"

  step 2 "clean git tree"
  local dirty
  dirty=$(git status --porcelain)
  if [[ -n "$dirty" && "$ALLOW_DIRTY" -ne 1 ]]; then
    echo "There are uncommitted changes. A deployment must match an exact commit." >&2
    echo "Pass --allow-dirty to force it. Status:" >&2
    git status --porcelain >&2
    exit 1
  fi
  if [[ -n "$dirty" ]]; then
    echo "    warning: dirty tree, continuing because of --allow-dirty"
  else
    echo "    commit $(git rev-parse --short HEAD), clean tree"
  fi
}

preflight
ADMIN_ADDR=$(identity_address "$STELLAR_ACCOUNT")

if [[ "$CHECK_ONLY" -eq 1 ]]; then
  echo
  echo "OK: --check-only finished (steps 1 and 2). Nothing was deployed."
  trap - ERR
  exit 0
fi

step 3 "make ci"
make ci
if [[ ! -f "$WASM" ]]; then
  die "${WASM} was not generated"
fi
echo "    wasm ${WASM}"

step 4 "stellar contract deploy"
raw_deploy=$(
  stellar contract deploy \
    --wasm "$WASM" \
    --source-account "$STELLAR_ACCOUNT" \
    --network "$STELLAR_NETWORK" \
    -- \
    --asset "$USDC_CONTRACT" \
    --owner "$ADMIN_ADDR" \
    --strategy-timelock "$STRATEGY_TIMELOCK" \
    --deposit-cap "$DEPOSIT_CAP"
)
if ! VAULT_ID=$(parse_contract_id "$raw_deploy"); then
  die "could not read the C… address from the deploy. Output: ${raw_deploy}"
fi
echo "    deployed ${VAULT_ID}"

step 5 "propose_strategy + wait + accept_strategy"
invoke_send "$STELLAR_ACCOUNT" "$VAULT_ID" propose_strategy \
  --strategy "$STRATEGY_CONTRACT" \
  --target-bps "$STRATEGY_TARGET_BPS"
wait_secs=$((STRATEGY_TIMELOCK + 10))
echo "    proposal sent; waiting ${wait_secs}s (timelock ${STRATEGY_TIMELOCK} + 10)"
countdown "$wait_secs"
invoke_send "$STELLAR_ACCOUNT" "$VAULT_ID" accept_strategy
echo "    strategy accepted"

step 6 "Fondex initial deposit ($(fmt_usdc "$INITIAL_DEPOSIT"))"
invoke_send "$STELLAR_ACCOUNT" "$VAULT_ID" deposit \
  --assets "$INITIAL_DEPOSIT" \
  --receiver "$ADMIN_ADDR" \
  --from "$ADMIN_ADDR"
echo "    deposit ok"

step 7 "verify state"
strategies_json=$(invoke_view "$VAULT_ID" strategies)
target=$(strategy_target_bps_from_json "$strategies_json")
if [[ "$target" != "$STRATEGY_TARGET_BPS" ]]; then
  die "strategies() does not contain ${STRATEGY_CONTRACT} with target ${STRATEGY_TARGET_BPS} (got '${target}'). JSON: ${strategies_json}"
fi
echo "    strategies(): ${STRATEGY_CONTRACT} target_bps=${target}"

total=$(json_scalar "$(invoke_view "$VAULT_ID" total_assets)")
if [[ "$total" != "$INITIAL_DEPOSIT" ]]; then
  die "total_assets()=${total}, expected ${INITIAL_DEPOSIT}"
fi
echo "    total_assets()=${total}"

paused=$(json_scalar "$(invoke_view "$VAULT_ID" paused)")
if [[ "$paused" != "false" ]]; then
  die "paused()=${paused}, expected false"
fi
echo "    paused()=${paused}"

step 8 "record in README.md"
append_readme_deploy_row
echo "    row added (not committed)"

step 9 "lines for ${APP_ENV_REL}"
echo "VITE_VAULT_CONTRACT_ID=${VAULT_ID}"
echo "VITE_USDC_CONTRACT_ID=${USDC_CONTRACT}"
if [[ "$UPDATE_APP" -eq 1 ]]; then
  answer=N
  if [[ -t 0 ]]; then
    read -r -p "Write to ${APP_ENV_REL}? [y/N] " answer
  else
    echo "stdin is not a TTY; not writing ${APP_ENV_REL} (an interactive 'y' is required)."
  fi
  if [[ "$answer" == "y" || "$answer" == "Y" ]]; then
    upsert_env_var "$APP_ENV" "VITE_VAULT_CONTRACT_ID" "$VAULT_ID"
    upsert_env_var "$APP_ENV" "VITE_USDC_CONTRACT_ID" "$USDC_CONTRACT"
    echo "    wrote ${APP_ENV}"
  else
    echo "    did not write ${APP_ENV_REL}"
  fi
fi

trap - ERR
echo
echo "Done. Vault on testnet: ${VAULT_ID}"
echo "E2E: ./scripts/e2e-testnet.sh ${VAULT_ID}"

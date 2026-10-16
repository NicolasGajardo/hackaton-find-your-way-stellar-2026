#!/usr/bin/env bash
# End-to-end test on testnet: a real user deposits, rebalances, withdraws
# (pulling from the Blend strategy) and redeems all of their shares.
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
# shellcheck source=lib/testnet.sh
source "${SCRIPT_DIR}/lib/testnet.sh"

usage() {
  cat <<EOF
Usage: ./scripts/e2e-testnet.sh <VAULT_ID>

User identity: ${E2E_IDENTITY} (created and funded with friendbot if missing).
Default deposit: $(fmt_usdc "$E2E_DEPOSIT"). Withdrawal: $(fmt_usdc "$E2E_WITHDRAW").
Override: E2E_DEPOSIT=… E2E_WITHDRAW=… ./scripts/e2e-testnet.sh C… (in units, 7 decimals)
EOF
}

if [[ $# -lt 1 ]]; then
  usage
  exit 1
fi
if [[ "$1" == "-h" || "$1" == "--help" ]]; then
  usage
  exit 0
fi

VAULT_ID="$1"
if [[ ! "$VAULT_ID" =~ ^C[A-Z0-9]{55}$ ]]; then
  die "invalid VAULT_ID: ${VAULT_ID}"
fi

cd "$ROOT"
load_testnet_env

FAILED=0
CHECKS=()

pass() {
  CHECKS+=("✅ $1")
  echo "    ✅ $1"
}

fail() {
  FAILED=1
  CHECKS+=("❌ $1")
  echo "    ❌ $1" >&2
}

warn() {
  CHECKS+=("⚠️  $1")
  echo "    ⚠️  $1"
}

print_summary() {
  echo
  echo "==> Summary"
  local line
  for line in "${CHECKS[@]}"; do
    echo "$line"
  done
  if [[ "$FAILED" -ne 0 ]]; then
    echo "E2E FAILED"
    exit 1
  fi
  echo "E2E OK"
}

share_balance() {
  json_scalar "$(invoke_contract no "$E2E_IDENTITY" "$VAULT_ID" balance --account "$E2E_ADDR")"
}

echo "==> E2E FondexVault  vault=${VAULT_ID}"

echo
echo "==> Step 1: prepare ${E2E_IDENTITY}"
require_cmd stellar jq
require_identity "$STELLAR_ACCOUNT"
ADMIN_ADDR=$(identity_address "$STELLAR_ACCOUNT")

if identity_exists "$E2E_IDENTITY"; then
  echo "    identity ${E2E_IDENTITY} already exists"
  stellar keys fund "$E2E_IDENTITY" --network "$STELLAR_NETWORK" || true
else
  echo "    creating ${E2E_IDENTITY} (--network ${STELLAR_NETWORK} --fund)"
  stellar keys generate "$E2E_IDENTITY" --network "$STELLAR_NETWORK" --fund
fi
E2E_ADDR=$(identity_address "$E2E_IDENTITY")
echo "    ${E2E_IDENTITY} = ${E2E_ADDR}"

echo "    change-trust --line ${USDC_ASSET}"
stellar tx new change-trust \
  --source-account "$E2E_IDENTITY" \
  --network "$STELLAR_NETWORK" \
  --line "$USDC_ASSET"

set +e
user_usdc=$(usdc_balance "$E2E_ADDR" "$E2E_IDENTITY")
rc=$?
set -e
if [[ $rc -eq 1 ]]; then
  die "could not read the USDC balance of ${E2E_IDENTITY}"
fi
echo "    ${E2E_IDENTITY} USDC: $(fmt_usdc "$user_usdc")"
if [[ "$user_usdc" -lt "$E2E_DEPOSIT" ]]; then
  need=$((E2E_DEPOSIT - user_usdc))
  echo "    admin transfers $(fmt_usdc "$need") (Blend) to ${E2E_IDENTITY}"
  invoke_send "$STELLAR_ACCOUNT" "$USDC_CONTRACT" transfer \
    --from "$ADMIN_ADDR" \
    --to "$E2E_ADDR" \
    --amount "$need"
  user_usdc=$(json_scalar "$(invoke_contract no "$E2E_IDENTITY" "$USDC_CONTRACT" balance --id "$E2E_ADDR")")
  echo "    ${E2E_IDENTITY} USDC now: $(fmt_usdc "$user_usdc")"
fi
if [[ "$user_usdc" -lt "$E2E_DEPOSIT" ]]; then
  die "${E2E_IDENTITY} still has less than $(fmt_usdc "$E2E_DEPOSIT") (Blend)"
fi
pass "user ready (trustline + $(fmt_usdc "$E2E_DEPOSIT"))"

usdc_before_deposit=$user_usdc

echo
echo "==> Step 2: deposit($(fmt_usdc "$E2E_DEPOSIT")) as ${E2E_IDENTITY}"
invoke_send "$E2E_IDENTITY" "$VAULT_ID" deposit \
  --assets "$E2E_DEPOSIT" \
  --receiver "$E2E_ADDR" \
  --from "$E2E_ADDR"
shares_after_deposit=$(share_balance)
echo "    shares: ${shares_after_deposit}"
if [[ "$shares_after_deposit" == "0" ]]; then
  fail "deposit: no shares were minted"
  print_summary
  exit 1
fi
pass "deposit($(fmt_usdc "$E2E_DEPOSIT"))"

echo
echo "==> Step 3: rebalance() signed by ${E2E_IDENTITY} (no admin needed)"
invoke_send "$E2E_IDENTITY" "$VAULT_ID" rebalance
pass "rebalance() as ${E2E_IDENTITY}"

echo
echo "==> Step 4: ~90% strategy / ~10% reserve (tolerance ${REBALANCE_TOLERANCE_UNITS} units)"
total=$(json_scalar "$(invoke_view "$VAULT_ID" total_assets)")
idle=$(json_scalar "$(invoke_view "$VAULT_ID" idle_assets)")
balances_json=$(invoke_view "$VAULT_ID" strategy_balances)
strat=$(strategy_balance_from_json "$balances_json")
if [[ -z "$strat" ]]; then
  strat=0
  fail "strategy_balances() does not include ${STRATEGY_CONTRACT}. JSON: ${balances_json}"
fi
expected_strat=$((total * STRATEGY_TARGET_BPS / BPS))
expected_idle=$((total * (BPS - STRATEGY_TARGET_BPS) / BPS))
diff_strat=$(abs_diff "$strat" "$expected_strat")
diff_idle=$(abs_diff "$idle" "$expected_idle")
echo "    total_assets=${total}  strategy=${strat} (expected ${expected_strat}, Δ ${diff_strat})"
echo "    idle_assets=${idle} (expected ${expected_idle}, Δ ${diff_idle})"
if [[ "$diff_strat" -le "$REBALANCE_TOLERANCE_UNITS" && "$diff_idle" -le "$REBALANCE_TOLERANCE_UNITS" ]]; then
  pass "~90/10 allocation (strategy Δ ${diff_strat}, reserve Δ ${diff_idle})"
else
  fail "~90/10 allocation (strategy Δ ${diff_strat}, reserve Δ ${diff_idle}, tolerance ${REBALANCE_TOLERANCE_UNITS})"
fi

echo
echo "==> Step 5: interest — total_assets every ${INTEREST_POLL_SECS:-15}s for ${INTEREST_WAIT_SECS:-120}s"
interest_wait="${INTEREST_WAIT_SECS:-120}"
interest_poll="${INTEREST_POLL_SECS:-15}"
t=0
first="$total"
rose=0
while [[ "$t" -le "$interest_wait" ]]; do
  cur=$(json_scalar "$(invoke_view "$VAULT_ID" total_assets)")
  echo "    t=${t}s  total_assets=${cur}"
  if [[ "$cur" -gt "$first" ]]; then
    rose=1
  fi
  if [[ "$t" -ge "$interest_wait" ]]; then
    break
  fi
  sleep "$interest_poll"
  t=$((t + interest_poll))
done
if [[ "$rose" -eq 1 ]]; then
  pass "interest: total_assets went up (${first} → ${cur})"
else
  warn "interest: total_assets did not go up in ${interest_wait}s (testnet interest depends on pool usage; not a failure)"
fi

echo
echo "==> Step 6: withdraw($(fmt_usdc "$E2E_WITHDRAW")) to ${E2E_IDENTITY} (exceeds the reserve)"
usdc_before_wd=$(json_scalar "$(invoke_contract no "$E2E_IDENTITY" "$USDC_CONTRACT" balance --id "$E2E_ADDR")")
strat_before=$(strategy_balance_from_json "$(invoke_view "$VAULT_ID" strategy_balances)")
strat_before="${strat_before:-0}"
invoke_send "$E2E_IDENTITY" "$VAULT_ID" withdraw \
  --assets "$E2E_WITHDRAW" \
  --receiver "$E2E_ADDR" \
  --owner "$E2E_ADDR"
usdc_after_wd=$(json_scalar "$(invoke_contract no "$E2E_IDENTITY" "$USDC_CONTRACT" balance --id "$E2E_ADDR")")
got=$((usdc_after_wd - usdc_before_wd))
echo "    USDC ${usdc_before_wd} → ${usdc_after_wd} (Δ ${got})"
if [[ "$got" -eq "$E2E_WITHDRAW" ]]; then
  pass "withdraw: received exactly $(fmt_usdc "$E2E_WITHDRAW")"
else
  fail "withdraw: received ${got} units, expected ${E2E_WITHDRAW}"
fi
strat_after=$(strategy_balance_from_json "$(invoke_view "$VAULT_ID" strategy_balances)")
strat_after="${strat_after:-0}"
echo "    strategy_balances ${strat_before} → ${strat_after}"
if [[ "$strat_after" -lt "$strat_before" ]]; then
  pass "withdraw: strategy balance went down (pulled from the strategy)"
else
  fail "withdraw: strategy balance did not go down (${strat_before} → ${strat_after})"
fi

echo
echo "==> Step 7: redeem all shares of ${E2E_IDENTITY}"
shares=$(share_balance)
echo "    shares to redeem: ${shares}"
if [[ "$shares" == "0" ]]; then
  fail "redeem: ${E2E_IDENTITY} has no shares"
  print_summary
  exit 1
fi
invoke_send "$E2E_IDENTITY" "$VAULT_ID" redeem \
  --shares "$shares" \
  --receiver "$E2E_ADDR" \
  --owner "$E2E_ADDR"
shares_left=$(share_balance)
echo "    shares left: ${shares_left}"
if [[ "$shares_left" == "0" ]]; then
  pass "redeem: share balance is 0"
else
  fail "redeem: ${shares_left} shares left"
fi
usdc_final=$(json_scalar "$(invoke_contract no "$E2E_IDENTITY" "$USDC_CONTRACT" balance --id "$E2E_ADDR")")
received_redeem=$((usdc_final - usdc_after_wd))
received_total=$((got + received_redeem))
echo "    USDC received: withdraw ${got} + redeem ${received_redeem} = ${received_total} units (deposited ${E2E_DEPOSIT})"
if [[ "$received_total" -ge "$E2E_DEPOSIT" ]]; then
  pass "total received $(fmt_usdc "$received_total") >= deposit $(fmt_usdc "$E2E_DEPOSIT")"
else
  # Slightly less can be rounding in the vault's favor; report it as a failure
  # only if it is further off than Blend's tolerance.
  short=$((E2E_DEPOSIT - received_total))
  if [[ "$short" -le "$REBALANCE_TOLERANCE_UNITS" ]]; then
    pass "total received $(fmt_usdc "$received_total") (Δ ${short} units from rounding, within tolerance)"
  else
    fail "total received $(fmt_usdc "$received_total") vs deposit $(fmt_usdc "$E2E_DEPOSIT") (${short} units short)"
  fi
fi
echo "    USDC balance: start $(fmt_usdc "$usdc_before_deposit") → end $(fmt_usdc "$usdc_final")"

print_summary

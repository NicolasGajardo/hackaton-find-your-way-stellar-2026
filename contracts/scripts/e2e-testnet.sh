#!/usr/bin/env bash
# Prueba de punta a punta en testnet: un usuario real deposita, rebalancea,
# retira (sacando de la estrategia Blend) y rescata todas sus cuotas.
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
# shellcheck source=lib/testnet.sh
source "${SCRIPT_DIR}/lib/testnet.sh"

usage() {
  cat <<EOF
Uso: ./scripts/e2e-testnet.sh <VAULT_ID>

Identidad del usuario: ${E2E_IDENTITY} (se crea con friendbot si no existe).
Depósito por defecto: ${E2E_DEPOSIT} (100 USDC). Retiro: ${E2E_WITHDRAW} (50 USDC).
Sobrescribir: E2E_DEPOSIT=… E2E_WITHDRAW=… ./scripts/e2e-testnet.sh C…
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
  die "VAULT_ID inválido: ${VAULT_ID}"
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
  echo "==> Resumen"
  local line
  for line in "${CHECKS[@]}"; do
    echo "$line"
  done
  if [[ "$FAILED" -ne 0 ]]; then
    echo "E2E FALLÓ"
    exit 1
  fi
  echo "E2E OK"
}

share_balance() {
  json_scalar "$(invoke_contract no "$E2E_IDENTITY" "$VAULT_ID" balance --account "$E2E_ADDR")"
}

echo "==> E2E FondexVault  vault=${VAULT_ID}"

echo
echo "==> Paso 1: preparar ${E2E_IDENTITY}"
require_cmd stellar jq git
require_identity "$STELLAR_ACCOUNT"
ADMIN_ADDR=$(identity_address "$STELLAR_ACCOUNT")

if identity_exists "$E2E_IDENTITY"; then
  echo "    identidad ${E2E_IDENTITY} ya existe"
  stellar keys fund "$E2E_IDENTITY" --network "$STELLAR_NETWORK" || true
else
  echo "    creando ${E2E_IDENTITY} (--network ${STELLAR_NETWORK} --fund)"
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
  die "no se pudo consultar el USDC de ${E2E_IDENTITY}"
fi
echo "    USDC de ${E2E_IDENTITY}: ${user_usdc}"
if [[ "$user_usdc" -lt "$E2E_DEPOSIT" ]]; then
  need=$((E2E_DEPOSIT - user_usdc))
  echo "    el admin transfiere ${need} USDC de Blend a ${E2E_IDENTITY}"
  invoke_send "$STELLAR_ACCOUNT" "$USDC_CONTRACT" transfer \
    --from "$ADMIN_ADDR" \
    --to "$E2E_ADDR" \
    --amount "$need"
  user_usdc=$(json_scalar "$(invoke_contract no "$E2E_IDENTITY" "$USDC_CONTRACT" balance --id "$E2E_ADDR")")
  echo "    USDC de ${E2E_IDENTITY} ahora: ${user_usdc}"
fi
if [[ "$user_usdc" -lt "$E2E_DEPOSIT" ]]; then
  die "${E2E_IDENTITY} sigue sin ${E2E_DEPOSIT} USDC de Blend"
fi
pass "usuario preparado (trustline + ${E2E_DEPOSIT} USDC)"

usdc_before_deposit=$user_usdc

echo
echo "==> Paso 2: deposit(${E2E_DEPOSIT}) como ${E2E_IDENTITY}"
invoke_send "$E2E_IDENTITY" "$VAULT_ID" deposit \
  --assets "$E2E_DEPOSIT" \
  --receiver "$E2E_ADDR" \
  --from "$E2E_ADDR"
shares_after_deposit=$(share_balance)
echo "    cuotas: ${shares_after_deposit}"
if [[ "$shares_after_deposit" == "0" ]]; then
  fail "depósito: cuotas > 0"
  print_summary
  exit 1
fi
pass "deposit(${E2E_DEPOSIT})"

echo
echo "==> Paso 3: rebalance() firmado por ${E2E_IDENTITY} (no hace falta el admin)"
invoke_send "$E2E_IDENTITY" "$VAULT_ID" rebalance
pass "rebalance() como ${E2E_IDENTITY}"

echo
echo "==> Paso 4: ~90% estrategia / ~10% reserva (tolerancia ${REBALANCE_TOLERANCE_UNITS})"
total=$(json_scalar "$(invoke_view "$VAULT_ID" total_assets)")
idle=$(json_scalar "$(invoke_view "$VAULT_ID" idle_assets)")
balances_json=$(invoke_view "$VAULT_ID" strategy_balances)
strat=$(strategy_balance_from_json "$balances_json")
if [[ -z "$strat" ]]; then
  strat=0
  fail "strategy_balances() no incluye ${STRATEGY_CONTRACT}. JSON: ${balances_json}"
fi
expected_strat=$((total * STRATEGY_TARGET_BPS / BPS))
expected_idle=$((total * (BPS - STRATEGY_TARGET_BPS) / BPS))
diff_strat=$(abs_diff "$strat" "$expected_strat")
diff_idle=$(abs_diff "$idle" "$expected_idle")
echo "    total_assets=${total}  strategy=${strat} (esperado ${expected_strat}, Δ ${diff_strat})"
echo "    idle_assets=${idle} (esperado ${expected_idle}, Δ ${diff_idle})"
if [[ "$diff_strat" -le "$REBALANCE_TOLERANCE_UNITS" && "$diff_idle" -le "$REBALANCE_TOLERANCE_UNITS" ]]; then
  pass "asignación ~90/10 (Δ estrategia ${diff_strat}, Δ reserva ${diff_idle})"
else
  fail "asignación ~90/10 (strategy Δ ${diff_strat}, idle Δ ${diff_idle}, tolerancia ${REBALANCE_TOLERANCE_UNITS})"
fi

echo
echo "==> Paso 5: interés — total_assets cada 15s durante 2 minutos"
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
  pass "interés: total_assets subió (${first} → ${cur})"
else
  warn "interés: total_assets no subió en ${interest_wait}s (el interés en testnet depende del uso del pool; no es fallo)"
fi

echo
echo "==> Paso 6: withdraw(${E2E_WITHDRAW}) hacia ${E2E_IDENTITY} (supera la reserva)"
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
  pass "withdraw: USDC subió exactamente ${E2E_WITHDRAW}"
else
  fail "withdraw: USDC subió ${got}, se esperaba ${E2E_WITHDRAW}"
fi
strat_after=$(strategy_balance_from_json "$(invoke_view "$VAULT_ID" strategy_balances)")
strat_after="${strat_after:-0}"
echo "    strategy_balances ${strat_before} → ${strat_after}"
if [[ "$strat_after" -lt "$strat_before" ]]; then
  pass "withdraw: strategy_balances bajó (salió de la estrategia)"
else
  fail "withdraw: strategy_balances no bajó (${strat_before} → ${strat_after})"
fi

echo
echo "==> Paso 7: redeem de todas las cuotas de ${E2E_IDENTITY}"
shares=$(share_balance)
echo "    cuotas a rescatar: ${shares}"
if [[ "$shares" == "0" ]]; then
  fail "redeem: ${E2E_IDENTITY} no tiene cuotas"
  print_summary
  exit 1
fi
invoke_send "$E2E_IDENTITY" "$VAULT_ID" redeem \
  --shares "$shares" \
  --receiver "$E2E_ADDR" \
  --owner "$E2E_ADDR"
shares_left=$(share_balance)
echo "    cuotas después: ${shares_left}"
if [[ "$shares_left" == "0" ]]; then
  pass "redeem: cuotas en 0"
else
  fail "redeem: quedaron ${shares_left} cuotas"
fi
usdc_final=$(json_scalar "$(invoke_contract no "$E2E_IDENTITY" "$USDC_CONTRACT" balance --id "$E2E_ADDR")")
received_redeem=$((usdc_final - usdc_after_wd))
received_total=$((got + received_redeem))
echo "    USDC recibido: withdraw ${got} + redeem ${received_redeem} = ${received_total} (depositó ${E2E_DEPOSIT})"
if [[ "$received_total" -ge "$E2E_DEPOSIT" ]]; then
  pass "USDC recibido en total ${received_total} >= depósito ${E2E_DEPOSIT}"
else
  # Un poco menos puede ser redondeo a favor de la bóveda; lo informamos como fallo
  # solo si se aleja más de la tolerancia de Blend.
  short=$((E2E_DEPOSIT - received_total))
  if [[ "$short" -le "$REBALANCE_TOLERANCE_UNITS" ]]; then
    pass "USDC recibido en total ${received_total} (Δ ${short} por redondeo, dentro de tolerancia)"
  else
    fail "USDC recibido en total ${received_total} vs depósito ${E2E_DEPOSIT} (faltan ${short})"
  fi
fi
echo "    saldo USDC inicial ${usdc_before_deposit} → final ${usdc_final}"

print_summary

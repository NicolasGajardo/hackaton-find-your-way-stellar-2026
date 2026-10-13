#!/usr/bin/env bash
# Despliega FondexVault en testnet, acepta la estrategia Blend USDC de
# DeFindex y deja un depósito inicial de Fondex. No commitea el README.
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
# shellcheck source=lib/testnet.sh
source "${SCRIPT_DIR}/lib/testnet.sh"

usage() {
  cat <<EOF
Uso: ./scripts/deploy-testnet.sh [flags]

  --check-only    Solo verificaciones previas y árbol git limpio (pasos 1 y 2)
  --allow-dirty   Permite desplegar con cambios sin commitear
  --update-app    Pregunta antes de escribir ${APP_ENV_REL}
  -h, --help      Esta ayuda

Variables: scripts/testnet.env (se pueden sobrescribir con VAR=…).
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
      echo "flag desconocido: $1" >&2
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
    echo "ERROR: el contrato ya está desplegado en ${VAULT_ID}." >&2
    echo "Falló en el paso ${CURRENT_STEP} (${CURRENT_STEP_NAME}). Podés continuar a mano desde ahí." >&2
  fi
  exit "$code"
}
trap on_err ERR

step() {
  CURRENT_STEP="$1"
  CURRENT_STEP_NAME="$2"
  echo
  echo "==> Paso ${CURRENT_STEP}: ${CURRENT_STEP_NAME}"
}

preflight() {
  step 1 "verificaciones previas"
  require_cmd stellar jq git make
  echo "    stellar $(stellar --version | head -1)"
  echo "    jq $(jq --version)"
  require_identity "$STELLAR_ACCOUNT"
  ADMIN_ADDR=$(identity_address "$STELLAR_ACCOUNT")
  echo "    identidad ${STELLAR_ACCOUNT} = ${ADMIN_ADDR}"

  local rc bal=0
  set +e
  bal=$(usdc_balance "$ADMIN_ADDR")
  rc=$?
  set -e
  if [[ $rc -eq 1 ]]; then
    die "no se pudo consultar el balance de USDC de Blend"
  fi
  if [[ $rc -eq 2 || "$bal" -lt "$INITIAL_DEPOSIT" ]]; then
    print_blend_usdc_help "$ADMIN_ADDR"
    if [[ $rc -eq 2 ]]; then
      echo "Detalle: la SAC devolvió error de trustline faltante (balance visto como 0)." >&2
    else
      echo "Balance actual: ${bal}" >&2
    fi
    exit 1
  fi
  echo "    USDC de Blend: ${bal} (>= ${INITIAL_DEPOSIT})"

  step 2 "árbol git limpio"
  local dirty
  dirty=$(git status --porcelain)
  if [[ -n "$dirty" && "$ALLOW_DIRTY" -ne 1 ]]; then
    echo "Hay cambios sin commitear. Un despliegue debe corresponder a un commit exacto." >&2
    echo "Pasá --allow-dirty para forzar. Estado:" >&2
    git status --porcelain >&2
    exit 1
  fi
  if [[ -n "$dirty" ]]; then
    echo "    aviso: árbol sucio, se continúa por --allow-dirty"
  else
    echo "    commit $(git rev-parse --short HEAD), árbol limpio"
  fi
}

preflight
ADMIN_ADDR=$(identity_address "$STELLAR_ACCOUNT")

if [[ "$CHECK_ONLY" -eq 1 ]]; then
  echo
  echo "OK: --check-only terminó (pasos 1 y 2). No se despliega."
  trap - ERR
  exit 0
fi

step 3 "make ci"
make ci
if [[ ! -f "$WASM" ]]; then
  die "no se generó ${WASM}"
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
  die "no se pudo leer la dirección C… del deploy. Salida: ${raw_deploy}"
fi
echo "    desplegado ${VAULT_ID}"

step 5 "propose_strategy + espera + accept_strategy"
invoke_send "$STELLAR_ACCOUNT" "$VAULT_ID" propose_strategy \
  --strategy "$STRATEGY_CONTRACT" \
  --target-bps "$STRATEGY_TARGET_BPS"
wait_secs=$((STRATEGY_TIMELOCK + 10))
echo "    propuesta enviada; esperando ${wait_secs}s (timelock ${STRATEGY_TIMELOCK} + 10)"
countdown "$wait_secs"
invoke_send "$STELLAR_ACCOUNT" "$VAULT_ID" accept_strategy
echo "    estrategia aceptada"

step 6 "depósito inicial de Fondex (${INITIAL_DEPOSIT})"
invoke_send "$STELLAR_ACCOUNT" "$VAULT_ID" deposit \
  --assets "$INITIAL_DEPOSIT" \
  --receiver "$ADMIN_ADDR" \
  --from "$ADMIN_ADDR"
echo "    deposit ok"

step 7 "verificar estado"
strategies_json=$(invoke_view "$VAULT_ID" strategies)
target=$(strategy_target_bps_from_json "$strategies_json")
if [[ "$target" != "$STRATEGY_TARGET_BPS" ]]; then
  die "strategies() no tiene ${STRATEGY_CONTRACT} con target ${STRATEGY_TARGET_BPS} (obtuve '${target}'). JSON: ${strategies_json}"
fi
echo "    strategies(): ${STRATEGY_CONTRACT} target_bps=${target}"

total=$(json_scalar "$(invoke_view "$VAULT_ID" total_assets)")
if [[ "$total" != "$INITIAL_DEPOSIT" ]]; then
  die "total_assets()=${total}, se esperaba ${INITIAL_DEPOSIT}"
fi
echo "    total_assets()=${total}"

paused=$(json_scalar "$(invoke_view "$VAULT_ID" paused)")
if [[ "$paused" != "false" ]]; then
  die "paused()=${paused}, se esperaba false"
fi
echo "    paused()=${paused}"

step 8 "registro en README.md"
append_readme_deploy_row
echo "    fila agregada (sin commit)"

step 9 "líneas para ${APP_ENV_REL}"
echo "VITE_VAULT_CONTRACT_ID=${VAULT_ID}"
echo "VITE_USDC_CONTRACT_ID=${USDC_CONTRACT}"
if [[ "$UPDATE_APP" -eq 1 ]]; then
  answer=N
  if [[ -t 0 ]]; then
    read -r -p "¿Escribir en ${APP_ENV_REL}? [s/N] " answer
  else
    echo "stdin no es un TTY; no se escribe ${APP_ENV_REL} (hace falta 's')."
  fi
  if [[ "$answer" == "s" || "$answer" == "S" ]]; then
    upsert_env_var "$APP_ENV" "VITE_VAULT_CONTRACT_ID" "$VAULT_ID"
    upsert_env_var "$APP_ENV" "VITE_USDC_CONTRACT_ID" "$USDC_CONTRACT"
    echo "    escrito ${APP_ENV}"
  else
    echo "    no se escribió ${APP_ENV_REL}"
  fi
fi

trap - ERR
echo
echo "Listo. Bóveda en testnet: ${VAULT_ID}"
echo "E2E: ./scripts/e2e-testnet.sh ${VAULT_ID}"

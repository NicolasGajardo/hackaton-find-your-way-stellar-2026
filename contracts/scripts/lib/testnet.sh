# shellcheck shell=bash
# Helpers compartidos por deploy-testnet.sh y e2e-testnet.sh.

_LIB_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
SCRIPTS_DIR=$(cd "${_LIB_DIR}/.." && pwd)
ROOT=$(cd "${SCRIPTS_DIR}/.." && pwd)
WASM="${ROOT}/target/wasm32v1-none/release/fondex_vault.wasm"
APP_ENV_REL="../app/.env.local"
APP_ENV="${ROOT}/${APP_ENV_REL}"
BLEND_FAUCET_URL="https://testnet.blend.capital"
BPS=10000
REBALANCE_TOLERANCE_UNITS="${REBALANCE_TOLERANCE_UNITS:-1000}"
INSTRUCTION_LEEWAY="${INSTRUCTION_LEEWAY:-100000000}"
E2E_IDENTITY="${E2E_IDENTITY:-e2e-user}"
E2E_DEPOSIT="${E2E_DEPOSIT:-1000000000}"
E2E_WITHDRAW="${E2E_WITHDRAW:-500000000}"

load_testnet_env() {
  local env_file="${SCRIPTS_DIR}/testnet.env"
  if [[ ! -f "$env_file" ]]; then
    die "no está ${env_file}"
  fi
  local line key val
  while IFS= read -r line || [[ -n "$line" ]]; do
    [[ "$line" =~ ^[[:space:]]*# ]] && continue
    [[ -z "${line//[[:space:]]/}" ]] && continue
    key="${line%%=*}"
    val="${line#*=}"
    if [[ -z "${!key:-}" ]]; then
      export "${key}=${val}"
    fi
  done < "$env_file"

  : "${STELLAR_ACCOUNT:?falta STELLAR_ACCOUNT}"
  : "${STELLAR_NETWORK:?falta STELLAR_NETWORK}"
  : "${USDC_CONTRACT:?falta USDC_CONTRACT}"
  : "${USDC_ASSET:?falta USDC_ASSET}"
  : "${STRATEGY_CONTRACT:?falta STRATEGY_CONTRACT}"
  : "${STRATEGY_TARGET_BPS:?falta STRATEGY_TARGET_BPS}"
  : "${STRATEGY_TIMELOCK:?falta STRATEGY_TIMELOCK}"
  : "${DEPOSIT_CAP:?falta DEPOSIT_CAP}"
  : "${INITIAL_DEPOSIT:?falta INITIAL_DEPOSIT}"
}

die() {
  echo "ERROR: $*" >&2
  if [[ -n "${VAULT_ID:-}" ]]; then
    echo "El contrato ya está desplegado en ${VAULT_ID}." >&2
    echo "Falló en el paso ${CURRENT_STEP:-?} (${CURRENT_STEP_NAME:-desconocido}). Podés continuar a mano desde ahí." >&2
  fi
  trap - ERR
  exit 1
}

require_cmd() {
  local cmd
  for cmd in "$@"; do
    if ! command -v "$cmd" >/dev/null 2>&1; then
      die "no está '${cmd}' en PATH"
    fi
  done
}

identity_exists() {
  local name="$1"
  stellar keys ls | grep -qx "$name"
}

identity_address() {
  stellar keys public-key "$1"
}

require_identity() {
  local name="$1"
  if ! identity_exists "$name"; then
    die "no existe la identidad '${name}' en stellar keys. Creala con: stellar keys generate ${name} --network ${STELLAR_NETWORK} --fund"
  fi
}

trim() {
  local s="$1"
  s="${s#"${s%%[![:space:]]*}"}"
  s="${s%"${s##*[![:space:]]}"}"
  printf '%s' "$s"
}

json_scalar() {
  local raw
  raw=$(trim "$1")
  if [[ -z "$raw" ]]; then
    echo "0"
    return
  fi
  jq -r 'if type == "string" then . else tostring end' <<<"$raw"
}

parse_contract_id() {
  local raw
  raw=$(trim "$1")
  local id=""
  id=$(jq -r . <<<"$raw" 2>/dev/null || true)
  if [[ "$id" =~ ^C[A-Z0-9]{55}$ ]]; then
    printf '%s\n' "$id"
    return 0
  fi
  id=$(grep -Eo 'C[A-Z0-9]{55}' <<<"$raw" | tail -1 || true)
  if [[ "$id" =~ ^C[A-Z0-9]{55}$ ]]; then
    printf '%s\n' "$id"
    return 0
  fi
  return 1
}

invoke_contract() {
  local send="$1"
  local source="$2"
  local contract="$3"
  local fn="$4"
  shift 4
  local cmd=(
    stellar contract invoke
    --id "$contract"
    --source-account "$source"
    --network "$STELLAR_NETWORK"
  )
  if [[ "$send" == "no" ]]; then
    cmd+=(--send=no)
  fi
  if [[ -n "${INSTRUCTION_LEEWAY:-}" ]]; then
    cmd+=(--instruction-leeway "$INSTRUCTION_LEEWAY")
  fi
  cmd+=(-- "$fn" "$@")
  "${cmd[@]}"
}

invoke_view() {
  invoke_contract no "$STELLAR_ACCOUNT" "$@"
}

invoke_send() {
  local source="$1"
  shift
  invoke_contract default "$source" "$@"
}

usdc_balance() {
  local account="$1"
  local source="${2:-$STELLAR_ACCOUNT}"
  local err out rc
  err=$(mktemp)
  set +e
  out=$(stellar contract invoke \
    --id "$USDC_CONTRACT" \
    --source-account "$source" \
    --network "$STELLAR_NETWORK" \
    --send=no \
    -- \
    balance --id "$account" 2>"$err")
  rc=$?
  set -e
  if [[ $rc -eq 0 ]]; then
    rm -f "$err"
    json_scalar "$out"
    return 0
  fi
  if grep -qi 'trustline entry is missing' "$err"; then
    rm -f "$err"
    echo "0"
    return 2
  fi
  cat "$err" >&2
  rm -f "$err"
  return "$rc"
}

print_blend_usdc_help() {
  local addr="$1"
  cat >&2 <<EOF
La cuenta ${STELLAR_ACCOUNT} (${addr}) no tiene al menos ${INITIAL_DEPOSIT} unidades
(1 USDC si INITIAL_DEPOSIT=10000000) del USDC de Blend:

  SAC    ${USDC_CONTRACT}
  activo ${USDC_ASSET}

Este no es el USDC de Circle en testnet (CBIELTK6…). Conseguilo en ${BLEND_FAUCET_URL}
conectando la misma cuenta; el faucet crea la trustline y acredita USDC de prueba.
EOF
}

abs_diff() {
  local a="$1" b="$2"
  if ((a >= b)); then
    echo $((a - b))
  else
    echo $((b - a))
  fi
}

strategy_target_bps_from_json() {
  local json="$1"
  jq -r --arg s "$STRATEGY_CONTRACT" '
    .[] | select(.address == $s) | .target_bps | tostring
  ' <<<"$json" | head -1
}

strategy_balance_from_json() {
  local json="$1"
  jq -r --arg s "$STRATEGY_CONTRACT" '
    def as_str: if type == "string" then . else tostring end;
    .[]
    | if type == "array" and (.[0] | tostring) == $s then .[1] | as_str
      elif type == "object" and ((.address // "") == $s) then (.balance // .[1] // .["1"]) | as_str
      else empty end
  ' <<<"$json" | head -1
}

countdown() {
  local secs="$1"
  local i
  for ((i = secs; i >= 1; i--)); do
    printf '\r    esperando %ss para accept_strategy... ' "$i"
    sleep 1
  done
  printf '\n'
}

upsert_env_var() {
  local file="$1" key="$2" value="$3"
  mkdir -p "$(dirname "$file")"
  if [[ -f "$file" ]] && grep -q "^${key}=" "$file"; then
    sed -i "s|^${key}=.*|${key}=${value}|" "$file"
  else
    if [[ -f "$file" && -s "$file" ]]; then
      local last
      last=$(tail -c 1 "$file" || true)
      if [[ -n "$last" && "$last" != $'\n' ]]; then
        printf '\n' >>"$file"
      fi
    fi
    printf '%s=%s\n' "$key" "$value" >>"$file"
  fi
}

append_readme_deploy_row() {
  local readme="${ROOT}/README.md"
  local heading="Despliegues FondexVault (testnet)"
  local date row
  date=$(date +%Y-%m-%d)
  row="| ${date} | \`${VAULT_ID}\` | Blend USDC, objetivo ${STRATEGY_TARGET_BPS} bps, depósito inicial ${INITIAL_DEPOSIT} unidades |"

  if [[ ! -f "$readme" ]]; then
    die "no está ${readme}"
  fi

  if ! grep -q "$heading" "$readme"; then
    cat >>"$readme" <<EOF

## ${heading}

| Fecha | Dirección | Nota |
|---|---|---|
${row}
EOF
    return
  fi

  local tmp
  tmp=$(mktemp)
  awk -v row="$row" -v heading="$heading" '
    BEGIN { in_table = 0; seen = 0 }
    index($0, heading) { seen = 1 }
    seen && /^\|/ { in_table = 1 }
    in_table && !/^\|/ {
      print row
      in_table = 0
      seen = 0
    }
    { print }
    END { if (in_table) print row }
  ' "$readme" >"$tmp"
  mv "$tmp" "$readme"
}

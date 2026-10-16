# shellcheck shell=bash
# Helpers shared by deploy-testnet.sh and e2e-testnet.sh.

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
    die "${env_file} not found"
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

  : "${STELLAR_ACCOUNT:?missing STELLAR_ACCOUNT}"
  : "${STELLAR_NETWORK:?missing STELLAR_NETWORK}"
  : "${USDC_CONTRACT:?missing USDC_CONTRACT}"
  : "${USDC_ASSET:?missing USDC_ASSET}"
  : "${STRATEGY_CONTRACT:?missing STRATEGY_CONTRACT}"
  : "${STRATEGY_TARGET_BPS:?missing STRATEGY_TARGET_BPS}"
  : "${STRATEGY_TIMELOCK:?missing STRATEGY_TIMELOCK}"
  : "${DEPOSIT_CAP:?missing DEPOSIT_CAP}"
  : "${INITIAL_DEPOSIT:?missing INITIAL_DEPOSIT}"
}

die() {
  echo "ERROR: $*" >&2
  # Only deploy-testnet.sh tracks steps; there a set VAULT_ID means it was deployed.
  if [[ -n "${VAULT_ID:-}" && -n "${CURRENT_STEP_NAME:-}" ]]; then
    echo "The contract is already deployed at ${VAULT_ID}." >&2
    echo "Failed at step ${CURRENT_STEP:-?} (${CURRENT_STEP_NAME:-unknown}). You can continue manually from there." >&2
  fi
  trap - ERR
  exit 1
}

require_cmd() {
  local cmd
  for cmd in "$@"; do
    if ! command -v "$cmd" >/dev/null 2>&1; then
      die "'${cmd}' not found in PATH"
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
    die "identity '${name}' not found in stellar keys. Create it with: stellar keys generate ${name} --network ${STELLAR_NETWORK} --fund"
  fi
}

# Formats smallest units (7 decimals) as USDC, e.g. 10000000 → "1 USDC".
fmt_usdc() {
  local units="$1" sign=""
  if [[ "$units" == -* ]]; then
    sign="-"
    units="${units#-}"
  fi
  local whole=$((10#$units / 10000000))
  local frac
  frac=$(printf '%07d' $((10#$units % 10000000)))
  frac="${frac%"${frac##*[!0]}"}"
  if [[ -n "$frac" ]]; then
    printf '%s%s.%s USDC' "$sign" "$whole" "$frac"
  else
    printf '%s%s USDC' "$sign" "$whole"
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
Account ${STELLAR_ACCOUNT} (${addr}) needs at least $(fmt_usdc "$INITIAL_DEPOSIT")
(INITIAL_DEPOSIT=${INITIAL_DEPOSIT} units) of Blend's test USDC:

  SAC    ${USDC_CONTRACT}
  asset  ${USDC_ASSET}

This is not Circle's testnet USDC (CBIELTK6…). Get it at ${BLEND_FAUCET_URL}
by connecting the same account; the faucet creates the trustline and credits test USDC.
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
    printf '\r    waiting %ss for accept_strategy... ' "$i"
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
  local heading="FondexVault deployments (testnet)"
  local date row
  date=$(date +%Y-%m-%d)
  row="| ${date} | \`${VAULT_ID}\` | Blend USDC, target ${STRATEGY_TARGET_BPS} bps, initial deposit ${INITIAL_DEPOSIT} units |"

  if [[ ! -f "$readme" ]]; then
    die "${readme} not found"
  fi

  if ! grep -q "$heading" "$readme"; then
    cat >>"$readme" <<EOF

## ${heading}

| Date | Address | Note |
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

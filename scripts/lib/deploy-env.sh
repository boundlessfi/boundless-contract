#!/usr/bin/env bash
# Helpers shared by the deploy scripts. Source this file; it defines functions
# only.

# Reads KEY=VALUE lines from a .env.deploy file without executing it, so a
# tampered file cannot run commands. One level of surrounding quotes is
# stripped; nothing is expanded.
load_env_deploy() {
  local file=$1 line key value n=0
  [[ -f "$file" ]] || return 0
  while IFS= read -r line || [[ -n "$line" ]]; do
    n=$((n + 1))
    line="${line%$'\r'}"
    [[ -z "${line//[[:space:]]/}" || "$line" =~ ^[[:space:]]*# ]] && continue
    if [[ ! "$line" =~ ^(export[[:space:]]+)?([A-Z_][A-Z0-9_]*)=(.*)$ ]]; then
      echo "error: $file:$n is not KEY=VALUE" >&2
      return 1
    fi
    key="${BASH_REMATCH[2]}"
    value="${BASH_REMATCH[3]}"
    if [[ "$value" =~ ^\"(.*)\"$ || "$value" =~ ^\'(.*)\'$ ]]; then
      value="${BASH_REMATCH[1]}"
    fi
    export "$key=$value"
  done < "$file"
}

require_network() {
  case "$1" in
    testnet | futurenet | mainnet) ;;
    *)
      echo "error: network must be one of testnet, futurenet, mainnet (got: $1)" >&2
      exit 1
      ;;
  esac
}

require_g_address() {
  if [[ ! "$2" =~ ^G[A-Z2-7]{55}$ ]]; then
    echo "error: $1 must be a G... account address (got: $2)" >&2
    exit 1
  fi
}

require_c_address() {
  if [[ ! "$2" =~ ^C[A-Z2-7]{55}$ ]]; then
    echo "error: $1 must be a C... contract address (got: $2)" >&2
    exit 1
  fi
}

# Prints one top-level string field of a deployment record.
record_field() {
  jq -er --arg k "$2" '.[$k] | strings' "$1"
}

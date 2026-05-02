#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SOLANA_DIR="$ROOT_DIR/solana"
STATE_DIR="$SOLANA_DIR/.localnet"
LEDGER_DIR="$STATE_DIR/ledger"
KEYPAIR_DIR="$STATE_DIR/keypairs"

mkdir -p "$LEDGER_DIR" "$KEYPAIR_DIR"

PROGRAM_KP="$KEYPAIR_DIR/habitat-settlement-program-keypair.json"
ORACLE_KP="$KEYPAIR_DIR/oracle-keypair.json"
USER_KP="$KEYPAIR_DIR/test-user-keypair.json"

ensure_keypair() {
  local keypair_path="$1"
  if [[ ! -f "$keypair_path" ]]; then
    solana-keygen new \
      --no-bip39-passphrase \
      --silent \
      --force \
      -o "$keypair_path" >/dev/null
  fi
}

airdrop_wallet() {
  local keypair_path="$1"
  local amount_sol="$2"
  local pubkey
  pubkey="$(solana address -k "$keypair_path")"
  solana airdrop "$amount_sol" "$pubkey" --url localhost >/dev/null
  echo "Airdropped ${amount_sol} SOL to ${pubkey}"
}

ensure_keypair "$PROGRAM_KP"
ensure_keypair "$ORACLE_KP"
ensure_keypair "$USER_KP"

if ! pgrep -f "solana-test-validator.*$LEDGER_DIR" >/dev/null 2>&1; then
  nohup solana-test-validator --ledger "$LEDGER_DIR" --reset >/tmp/habitat-solana-validator.log 2>&1 &
  sleep 2
fi

solana config set --url localhost >/dev/null

airdrop_wallet "$ORACLE_KP" 10
airdrop_wallet "$USER_KP" 10

echo "Local validator bootstrap complete"
echo "Program keypair: $PROGRAM_KP"
echo "Oracle keypair:  $ORACLE_KP"
echo "User keypair:    $USER_KP"

#!/bin/bash
# Exit immediately if any command fails
set -e

# Resolve script directory to allow running this script from anywhere
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
cd "$SCRIPT_DIR"

# Determine the CLI tool to use (favoring soroban, falling back to stellar or direct cargo bin paths)
if command -v soroban &> /dev/null; then
  CLI="soroban"
elif [[ -x "$HOME/.cargo/bin/soroban" ]]; then
  CLI="$HOME/.cargo/bin/soroban"
elif command -v stellar &> /dev/null; then
  CLI="stellar"
elif [[ -x "$HOME/.cargo/bin/stellar" ]]; then
  CLI="$HOME/.cargo/bin/stellar"
else
  echo "Error: Neither 'soroban' nor 'stellar' CLI could be found." >&2
  echo "Please install the Stellar/Soroban CLI and make sure it is in your PATH." >&2
  exit 1
fi

# Configuration overrides (can be passed via environment variables)
SOURCE_ACCOUNT="${STELLAR_SOURCE_ACCOUNT:-deployer}"
NETWORK="${STELLAR_NETWORK:-testnet}"

# Parse command line options
while [[ $# -gt 0 ]]; do
  case $1 in
    -s|--source)
      SOURCE_ACCOUNT="$2"
      shift 2
      ;;
    -n|--network)
      NETWORK="$2"
      shift 2
      ;;
    -h|--help)
      echo "Usage: $0 [options]"
      echo ""
      echo "Options:"
      echo "  -s, --source <account>  Source account/identity (default: $SOURCE_ACCOUNT)"
      echo "  -n, --network <network> Network to deploy to (default: $NETWORK)"
      echo "  -h, --help              Show this help message"
      exit 0
      ;;
    *)
      echo "Unknown option: $1" >&2
      exit 1
      ;;
  esac
done

echo "============================================="
echo "Building Kolo Smart Contract..."
echo "============================================="
cargo build --target wasm32-unknown-unknown --release

echo ""
echo "============================================="
echo "Optimizing WASM Binary..."
echo "============================================="
# Run the optimization command as requested in the acceptance criteria
"$CLI" contract optimize --wasm target/wasm32-unknown-unknown/release/kolo_savings_group.wasm

echo ""
echo "============================================="
echo "Deploying to Stellar Network: $NETWORK"
echo "============================================="

# Fund the account if deploying to a test network to ensure it is active and has gas money
if [[ "$NETWORK" == "testnet" || "$NETWORK" == "futurenet" ]]; then
  echo "Funding account '$SOURCE_ACCOUNT' on network '$NETWORK'..."
  "$CLI" keys fund "$SOURCE_ACCOUNT" --network "$NETWORK" || echo "Note: Account funding skipped or failed (might already be funded)."
fi

echo "Deploying contract with identity '$SOURCE_ACCOUNT'..."
CONTRACT_ID=$("$CLI" contract deploy \
  --wasm target/wasm32-unknown-unknown/release/kolo_savings_group.optimized.wasm \
  --source "$SOURCE_ACCOUNT" \
  --network "$NETWORK")

echo ""
echo "============================================="
echo "Deployment Successful!"
echo "Contract ID: $CONTRACT_ID"
echo "============================================="

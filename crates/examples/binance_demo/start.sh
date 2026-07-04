#!/bin/bash

# Binance KYC Demo 启动脚本
# 使用方法:
#   ./start.sh verifier    # 启动 Verifier
#   ./start.sh prover      # 启动 Prover

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

cd "$PROJECT_ROOT"

case "$1" in
    verifier)
        echo "🚀 Starting Verifier..."
        VERIFIER_ADDR="${VERIFIER_ADDR:-127.0.0.1:8080}" \
        cargo run --release --bin binance_verifier
        ;;
    prover)
        echo "🚀 Starting Prover..."
        VERIFIER_URL="${VERIFIER_URL:-ws://127.0.0.1:8080}" \
        TARGET_URL="${TARGET_URL:-https://www.binance.com/setting/kyc}" \
        cargo run --release --bin binance_prover
        ;;
    *)
        echo "Usage: $0 {verifier|prover}"
        echo ""
        echo "Examples:"
        echo "  $0 verifier                    # Start verifier on 127.0.0.1:8080"
        echo "  VERIFIER_ADDR=0.0.0.0:8080 $0 verifier  # Start on all interfaces"
        echo "  $0 prover                      # Start prover with default settings"
        echo "  TARGET_URL=https://example.com $0 prover  # Connect to different URL"
        exit 1
        ;;
esac

#!/bin/sh
# Run inside an isolated Rust 1.99 Debian container with this repository mounted.
set -eu
export DEBIAN_FRONTEND=noninteractive
apt-get update -qq
apt-get install -y -qq nodejs python3 pkg-config libssl-dev
export CARGO_TARGET_DIR=/work/.local/linux-target
export CARGO_BUILD_JOBS=1
node scripts/managed-cargo.mjs build --workspace --locked
node scripts/managed-cargo.mjs build -p lapis-ai-runtime --examples --locked
node scripts/managed-cargo.mjs test --workspace --locked
python3 scripts/client-acceptance.py --acp "$CARGO_TARGET_DIR/debug/lapis-acp" --agent "$CARGO_TARGET_DIR/debug/lapis-agent" --tools "$CARGO_TARGET_DIR/debug/lapis-agent-tools" --client "$CARGO_TARGET_DIR/debug/examples/native-session" --catalog-client "$CARGO_TARGET_DIR/debug/examples/agent-catalog" --allow
python3 scripts/client-acceptance.py --acp "$CARGO_TARGET_DIR/debug/lapis-acp" --agent "$CARGO_TARGET_DIR/debug/lapis-agent" --tools "$CARGO_TARGET_DIR/debug/lapis-agent-tools" --client "$CARGO_TARGET_DIR/debug/examples/native-session"
python3 scripts/local-compute.acceptance.py --compute "$CARGO_TARGET_DIR/debug/lapis-compute" --root "${LAPIS_AI_NATIVE_ACCEPTANCE_ROOT:-/work/.local/linux-trixie-native}" --cold --acp "$CARGO_TARGET_DIR/debug/lapis-acp" --agent "$CARGO_TARGET_DIR/debug/lapis-agent" --tools "$CARGO_TARGET_DIR/debug/lapis-agent-tools"

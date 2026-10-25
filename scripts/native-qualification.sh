#!/usr/bin/env bash
# Cold acceptance on the two admitted ARM64 native platforms.
set -euo pipefail
export LAPIS_AI_VALIDATION_RUNTIME=native
profile=${1:-debug}
test "$profile" = debug || test "$profile" = release
if [ "$profile" = debug ]; then
  pnpm build
else
  pnpm dev:cargo -- build --workspace --release --locked
  pnpm dev:cargo -- build -p lapis-ai-runtime --examples --release --locked
fi
bin="target/$profile"
root=".local/ci-native-$profile"
bundle="dist/qualified-$profile"
python3 scripts/client-acceptance.py --acp "$bin/lapis-acp" --agent "$bin/lapis-agent" --tools "$bin/lapis-agent-tools" --client "$bin/examples/native-session" --catalog-client "$bin/examples/agent-catalog" --allow
python3 scripts/client-acceptance.py --acp "$bin/lapis-acp" --agent "$bin/lapis-agent" --tools "$bin/lapis-agent-tools" --client "$bin/examples/native-session"
python3 scripts/local-compute.acceptance.py --compute "$bin/lapis-compute" --root "$root" --cold --acp "$bin/lapis-acp" --agent "$bin/lapis-agent" --tools "$bin/lapis-agent-tools"
native_manifest=$(find "$root/native-runtime" -name manifest.json -type f -print -quit)
test -n "$native_manifest"
pnpm package:stage -- --profile "$profile" --destination "$bundle" --native-runtime "$(dirname "$native_manifest")"
python3 scripts/client-acceptance.py --acp "$bundle/lapis-acp" --agent "$bundle/lapis-agent" --tools "$bundle/lapis-agent-tools" --client "$bin/examples/native-session" --bundle "$bundle" --catalog-client "$bin/examples/agent-catalog" --allow
python3 scripts/local-compute.acceptance.py --compute "$bundle/lapis-compute" --root ".local/ci-bundled-$profile" --cold --native-runtime "$bundle/native-runtime" --acp "$bundle/lapis-acp" --agent "$bundle/lapis-agent" --tools "$bundle/lapis-agent-tools"
python3 scripts/compute-client-acceptance.py --client "$bin/examples/native-compute" --compute "$bundle/lapis-compute" --native-runtime "$bundle/native-runtime" --root ".local/ci-rust-client-$profile" --bundle "$bundle"

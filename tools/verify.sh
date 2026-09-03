#!/usr/bin/env bash
# Local gate, mirroring CI. The golden must not move across a refactor.
set -uo pipefail
GOLDEN=0x534dd0e5200e8e4a
cd "$(dirname "$0")/.."
out=$(cargo run --release -q -p toile-cli -- drape 2>/dev/null | tail -1)
if [ "$out" = "$GOLDEN" ]; then echo "✓ golden $out"; else echo "✗ golden CAMBIÓ: $out"; exit 1; fi
cargo +nightly fmt --all --check >/dev/null 2>&1 && echo "✓ fmt" || { echo "✗ fmt"; exit 1; }
./tools/check-style.sh >/dev/null || { echo "✗ estilo"; ./tools/check-style.sh; exit 1; }
echo "✓ estilo"
cargo clippy --workspace --all-targets -q -- -D warnings >/dev/null 2>&1 && echo "✓ clippy" || { echo "✗ clippy"; exit 1; }
cargo test -q --workspace >/dev/null 2>&1 && echo "✓ tests" || { echo "✗ tests"; exit 1; }

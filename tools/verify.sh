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
if command -v cargo-deny >/dev/null 2>&1; then
  cargo deny check licenses bans sources >/dev/null 2>&1 && echo "✓ deny" || { echo "✗ deny"; cargo deny check licenses bans sources; exit 1; }
  cargo deny check advisories >/dev/null 2>&1 && echo "✓ advisories" || echo "· advisories: hay avisos (no bloquea)"
else echo "· deny: no instalado (brew install cargo-deny)"; fi
if command -v typos >/dev/null 2>&1; then
  typos >/dev/null 2>&1 && echo "✓ typos" || { echo "✗ typos"; typos; exit 1; }
else echo "· typos: no instalado (brew install typos-cli)"; fi
if cargo machete --version >/dev/null 2>&1; then
  cargo machete >/dev/null 2>&1 && echo "✓ machete" || { echo "✗ machete"; cargo machete; exit 1; }
else echo "· machete: no instalado (cargo install cargo-machete --locked)"; fi

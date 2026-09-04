#!/usr/bin/env bash
# Project rules that neither rustfmt nor clippy can see (docs/style.html).
# Uses only grep and find, so it runs anywhere CI puts a shell.
set -uo pipefail
cd "$(dirname "$0")/.."
export LC_ALL=en_US.UTF-8
fail=0
say() { printf '\033[31m✗ %s\033[0m\n' "$1"; fail=1; }

COMMENT='^[[:space:]]*(//|///|//!)'
TOML_COMMENT='^[[:space:]]*#'
WORD='(^|[^[:alnum:]])(que|para|los|las|una|este|esta|como|pero|porque|cuando|donde|desde|hasta|entre)([^[:alnum:]]|$)'
SPANISH="([áéíóúñÁÉÍÓÚÑ¿¡]|$WORD)"

# 1. Comments go in English, in the manifests too. Spanish diacritics and
#    inverted punctuation are unambiguous; a short stop-word list catches the
#    unaccented rest. Plain non-ASCII is NOT an error: em dashes and
#    multiplication signs belong in English prose too.
if grep -rnE --include='*.rs' "$COMMENT.*$SPANISH" crates ||
   grep -rnE --include='Cargo.toml' "$TOML_COMMENT.*$SPANISH" crates; then
  say "comentarios en español: el código va en inglés (§3)"
fi

# 2. A module doc belongs on a crate root, and stays short.
for f in $(grep -rlE --include='*.rs' '^//!' crates); do
  n=$(grep -c '^//!' "$f")
  case "$f" in
    */src/lib.rs) [ "$n" -gt 4 ] && say "$f: doc de crate de $n líneas (máx 4) (§2.3)" ;;
    *)            say "$f: //! fuera de un lib.rs (§2.3)" ;;
  esac
done

# 3. Provenance belongs in the commit message, not the source.
PROV='(issue #[0-9]|[Ss]pike [0-9]|§)'
if grep -rnE --include='*.rs' "$COMMENT.*$PROV" crates ||
   grep -rnE --include='Cargo.toml' "$TOML_COMMENT.*$PROV" crates; then
  say "procedencia en comentarios: eso va en el mensaje de commit (§2.2)"
fi

# 4. File size limit.
for f in $(find crates -name '*.rs'); do
  n=$(wc -l < "$f")
  [ "$n" -gt 300 ] && say "$f: $n líneas (máx 300) (§4.2)"
done

[ $fail -eq 0 ] && echo "✓ estilo"
exit $fail

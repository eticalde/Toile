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

# 5. A comment block that runs past this stops being a comment and becomes a
#    document — and a document nobody can see grows a changelog. 17 is the
#    longest block in the tree that earns its length on the merits (the
#    winding argument in anny_bake/rings/intersect.rs), so it is the cap.
#    Fenced blocks inside a doc are exempt: a byte-layout table or an
#    example is not prose, and shortening it says less.
BLOCK_MAX=17
for f in $(find crates -name '*.rs'); do
  run=0; start=0; fence=0; line=0
  while IFS= read -r l || [ -n "$l" ]; do
    line=$((line + 1))
    case "$l" in
      *[!' 	']*) body=${l#"${l%%[!' 	']*}"} ;;
      *) body= ;;
    esac
    case "$body" in
      '///'*|'//!'*|'//'*)
        [ "$run" -eq 0 ] && { start=$line; fence=0; }
        run=$((run + 1))
        text=${body#"${body%%[!/!]*}"}
        text=${text# }
        case "$text" in '```'*) fence=$((1 - fence)); run=$((run - 1)) ;;
                        *) [ "$fence" -eq 1 ] && run=$((run - 1)) ;;
        esac
        ;;
      *)
        [ "$run" -gt "$BLOCK_MAX" ] &&
          say "$f:$start: comentario de $run líneas seguidas (máx $BLOCK_MAX) (§2.4)"
        run=0
        ;;
    esac
  done < "$f"
  [ "$run" -gt "$BLOCK_MAX" ] &&
    say "$f:$start: comentario de $run líneas seguidas (máx $BLOCK_MAX) (§2.4)"
done

# 6. Only `+ - * / sqrt` and one pinned `libm` reach the geometry. Std's
#    transcendentals have unspecified precision, so the same source can give
#    different bits on ARM and x86 and a golden would move under a dependency
#    nobody touched. Anchored to method syntax, so the qualified `libm::acos`
#    this exists to permit does not match, and so `cos` does not match `acos`
#    or half the English language. Each file is read up to its own test
#    module: a test may measure with whatever it likes.
BANNED='\.(powf|powi|sin|cos|tan|asin|acos|atan|atan2|exp|exp2|ln|log2|log10|sinh|cosh|tanh|hypot)[[:space:]]*\('
GEOMETRY='crates/toile-sim/src crates/toile-engine/src crates/toile-anny/src crates/toile-mesh/src crates/toile-seamly/src crates/toile-geom/src'
for f in $(find $GEOMETRY -name '*.rs' | grep -v -e '/tests/' -e 'tests\.rs$' -e '/export/'); do
  hits=$(awk '/#\[cfg\(test\)\]/ { exit } { print FNR": "$0 }' "$f" | grep -E "$BANNED" | head -3)
  [ -n "$hits" ] && say "$f: trascendente de std en la geometría (§determinismo)
$hits"
done

# 6b. And libm is the exception only where an angle or a ramp earns it: a new
#     file reaching for it is a new way for two machines to disagree.
LIBM_EARNED='body/bake/pseudo\.rs|couture/seam\.rs|couture/place\.rs|toile-seamly/src/eval/angle\.rs'
for f in $(grep -rlE --include='*.rs' 'libm::' crates | grep -v -e '/tests/' -e 'tests\.rs$'); do
  echo "$f" | grep -qE "$LIBM_EARNED" ||
    say "$f: libm:: fuera de donde está justificado (§determinismo)"
done

# 7. One layout for a module with submodules: `foo.rs` beside `foo/`. Two
#    layouts means two places to look for the same thing, and the one that
#    names the module in its own filename is the one the tree uses.
if find crates -name 'mod.rs' | grep .; then
  say "mod.rs: el submódulo va en foo.rs junto a foo/ (§4.3)"
fi

# 8. An edit the document can apply is an edit a press can ask for. A command
#    reachable from no gesture is a feature nobody can use, and it passes every
#    other rule in this file: it compiles, formats and hashes exactly like the
#    working kind. Three separate reviews found one of these by accident and
#    none on purpose, twice in the commit that introduced it.
#    A command deliberately ahead of its tool goes on the list below with the
#    reason, because arguing for it is the part that was skipped.
#    Each name below is an edit with no door yet, and the reason it is allowed
#    to have none. A name leaves this list by growing a gesture, never by being
#    forgotten: SetGrain and the three label edits are owed to a person by a
#    decision already taken, so their lines say so and their absence is a debt.
AHEAD='SetPin|ClearPin'      # the datum enters before the gesture: PLAN-001 8.1
AHEAD="$AHEAD|SetGrain"      # the grain is read on every piece and turned on none
AHEAD="$AHEAD|LabelPoint"    # a node shows its own number and takes no name
AHEAD="$AHEAD|ShowLabel"     # so the label layer can only answer the pointer
AHEAD="$AHEAD|LabelLine"     # and an internal line's note is «—» for its life
AHEAD="$AHEAD|RemoveMannequin" # a product takes a body and never gives one back
KIND=crates/toile-doc/src/command/kind.rs
for v in $(awk '/^pub enum Command \{/ { on = 1; next } on && /^\}/ { exit } on' "$KIND" |
           grep -oE '^    [A-Z][A-Za-z]+ ?[{,(]' | tr -d ' {,('); do
  echo "$v" | grep -qE "^($AHEAD)$" && continue
  grep -rlE --include='*.rs' "Command::$v\b" crates/toile-app/src crates/toile-engine/src |
    grep -vE '(/tests?/|tests\.rs$)' | grep -q . ||
    say "Command::$v: ninguna pulsación lo pide (§3)"
done

[ $fail -eq 0 ] && echo "✓ estilo"
exit $fail

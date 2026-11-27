#!/usr/bin/env bash
# Differential check of one grammar against the original C, using the c2rs harness:
# every table, the lexers and scanners called directly on a mock lexer for every state, token soup,
# and mutation, prefix and incremental runs over the golden inputs.
#   usage: C2RS=<c2rs checkout, built> KEEP_WORK=<dir from tools/regen.sh> tools/verify.sh <grammar>
set -euo pipefail
G=${1:?grammar}
HERE=$(cd "$(dirname "$0")/.." && pwd)
C2RS=${C2RS:?path to a c2rs checkout}
W=${KEEP_WORK:?the work dir used by tools/regen.sh}
REG=$(ls -d "${CARGO_HOME:-$HOME/.cargo}"/registry/src/*/ | head -1)

case "$G" in
  python)     CSRC="$REG/tree-sitter-python-0.25.0/src" ;;
  typescript) CSRC="$W/typescript/typescript/src"; export C_INC="$W/typescript/common" ;;
  tsx)        CSRC="$W/tsx/tsx/src";               export C_INC="$W/tsx/common" ;;
  lua)        CSRC="$REG/tree-sitter-lua-0.5.0/src" ;;
  *)          CSRC="$W/$G/src" ;;
esac
[ -f "$CSRC/parser.c" ] || { echo "no C source for $G at $CSRC" >&2; exit 1; }

PRE=$(mktemp -d)
cp "$HERE/src/abi.rs" "$PRE/abi.rs"
cp "$HERE/src/$G/parser.rs" "$PRE/parser.rs"
if [ -f "$HERE/src/$G/scanner.rs" ]; then export RS_SCANNER="$HERE/src/$G/scanner.rs"; fi

export LITE_DIR="$W" PREGEN="$PRE" C2RS
export INPUTS_GOLDEN="$HERE/tests/golden/$G.golden" DIRECT=1 SOUP="${SOUP:-8000}" FUZZ="${FUZZ:-10}" PREFIX="${PREFIX:-12}" INCR="${INCR:-5}"
OUT=$("$C2RS/run.sh" "$G" "$CSRC" - - x 2>&1) || true
echo "$OUT"
echo "$OUT" | grep -q "RESULT: PASS" || { echo "verify: $G FAILED" >&2; exit 1; }

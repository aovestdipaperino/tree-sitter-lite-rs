#!/usr/bin/env bash
# Differential check of one grammar against the original C, using the c2rs harness:
# every table, the lexers and scanners called directly on a mock lexer for every state, token soup,
# and mutation, prefix and incremental runs over the golden inputs.
#   usage: C2RS=<c2rs checkout, built> KEEP_WORK=<dir from tools/regen.sh> tools/verify.sh <grammar>
set -euo pipefail
G=${1:?grammar}
# The symbol prefix the grammar exports when it differs from the module name (dart: dart_orchard).
X=${2:-$G}
HERE=$(cd "$(dirname "$0")/.." && pwd)
C2RS=${C2RS:?path to a c2rs checkout}
W=${KEEP_WORK:?the work dir used by tools/regen.sh}
REG=$(ls -d "${CARGO_HOME:-$HOME/.cargo}"/registry/src/*/ | head -1)

case "$G" in
  python)     CSRC="$REG/tree-sitter-python-0.25.0/src" ;;
  typescript) CSRC="$W/typescript/typescript/src"; export C_INC="$W/typescript/common" ;;
  tsx)        CSRC="$W/tsx/tsx/src";               export C_INC="$W/tsx/common" ;;
  dart)       CSRC="$W/dart/src" ;;
  lua)        CSRC="$REG/tree-sitter-lua-0.5.0/src" ;;
  cpp)        CSRC="$W/cpp/src" ;;
  scala)      CSRC="$REG/tree-sitter-scala-0.25.1/src" ;;
  kotlin)     CSRC="$W/kotlin/src" ;;
  ruby)       CSRC="$W/ruby/src" ;;
  c_sharp)    CSRC="$REG/tree-sitter-c-sharp-0.23.5/src" ;;
  bash)       CSRC="$REG/tree-sitter-bash-0.25.1/src" ;;
  php)        CSRC="$W/php/php/src";               export C_INC="$W/php/common" ;;
  swift)      CSRC="$REG/tree-sitter-swift-0.7.4/src" ;;
  qbasic)     CSRC="$REG/tree-sitter-qbasic-0.2.0/grammar-src" ;;
  zig)        CSRC="$W/zig/src" ;;
  pascal)     CSRC="$W/pascal/src" ;;
  objc)       CSRC="$W/objc/src" ;;
  vbnet)      CSRC="$REG/tree-sitter-vb-dotnet-0.1.0/src" ;;
  batch)      CSRC="$REG/tree-sitter-batch-0.3.2/src" ;;
  msbasic2)   CSRC="$REG/tree-sitter-msbasic2-0.2.0/grammar-src" ;;
  gwbasic)    CSRC="$REG/tree-sitter-gwbasic-0.2.0/grammar-src" ;;
  nix)        CSRC="$W/nix/src" ;;
  glsl)       CSRC="$W/glsl/src" ;;
  clojure)    CSRC="$W/clojure/src" ;;
  toml)       CSRC="$W/toml/src" ;;
  powershell) CSRC="$REG/tree-sitter-powershell-0.26.4/src" ;;
  quint)      CSRC="$HERE/tools/vendored/tree-sitter-quint-5155d17/src" ;;
  ocaml)      CSRC="$W/ocaml/grammars/ocaml/src"; export C_INC="$W/ocaml/common" ;;
  erlang)     CSRC="$W/erlang/src" ;;
  sql)        CSRC="$W/sql/src" ;;
  fsharp)     CSRC="$W/fsharp/fsharp/src"; export C_INC="$W/fsharp/common" ;;
  lean)       CSRC="$W/lean/grammar/src" ;;
  julia)      CSRC="$W/julia/src" ;;
  *)          CSRC="$W/$G/src" ;;
esac
[ -f "$CSRC/parser.c" ] || { echo "no C source for $G at $CSRC" >&2; exit 1; }

PRE=$(mktemp -d)
cp "$HERE/src/abi.rs" "$PRE/abi.rs"
cp "$HERE/src/$G/parser.rs" "$PRE/parser.rs"
if [ -f "$HERE/src/$G/scanner.rs" ]; then export RS_SCANNER="$HERE/src/$G/scanner.rs"; fi

export LITE_DIR="$W" PREGEN="$PRE" C2RS
export INPUTS_GOLDEN="$HERE/tests/golden/$G.golden" DIRECT=1 SOUP="${SOUP:-8000}" FUZZ="${FUZZ:-10}" PREFIX="${PREFIX:-12}" INCR="${INCR:-5}"
OUT=$("$C2RS/run.sh" "$X" "$CSRC" - - x 2>&1) || true
echo "$OUT"
echo "$OUT" | grep -q "RESULT: PASS" || { echo "verify: $G FAILED" >&2; exit 1; }

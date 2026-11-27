#!/usr/bin/env python3
"""Register one language in the shared files, idempotently (checklist step 4 in CONTRIBUTING.md).

  tools/register.py <name> --tier medium|full --const LANGUAGE [--crate tree-sitter-x-1.2.3]
                    [--mode emit|regen] [--subdir php]

`name` is the module name (`c_sharp`); the feature is `lang-<name>`. `--const` is the constant of
`src/<name>/mod.rs` that `language_<name>()` returns. `--crate`, `--mode` and `--subdir` add the
`tools/regen.sh` line (`emit`: the shipped parser.c is already ABI 15 and used as is; `regen`:
regenerate at ABI 15 first) and the `tools/verify.sh` case. Run from anywhere; safe to run twice.
"""
import argparse, pathlib, re

root = pathlib.Path(__file__).resolve().parent.parent
ap = argparse.ArgumentParser()
ap.add_argument("name")
ap.add_argument("--tier", required=True, choices=["medium", "full"])
ap.add_argument("--const", default="LANGUAGE")
ap.add_argument("--crate")
ap.add_argument("--mode", choices=["emit", "regen"], default="emit")
ap.add_argument("--subdir")
a = ap.parse_args()
n, feat = a.name, f"lang-{a.name}"


def edit(path, fn):
    p = root / path
    old = p.read_text()
    new = fn(old)
    if new != old:
        p.write_text(new)


def cargo(s):
    if re.search(rf"^{feat} = \[\]$", s, re.M):
        return s
    s = re.sub(rf"^({a.tier} = \[)(.*?)\]$", lambda m: f'{m.group(1)}{m.group(2)}, "{feat}"]', s, count=1, flags=re.M)
    last = list(re.finditer(r"^lang-[a-z0-9_-]+ = \[\]$", s, re.M))[-1]
    return s[: last.end()] + f"\n{feat} = []" + s[last.end() :]


def lib(s):
    if f"pub mod {n};" in s:
        return s
    g = f'#[cfg(feature = "{feat}")]\n'
    s = s.replace("\nuse tree_sitter_language::LanguageFn;", f"{g}pub mod {n};\n\nuse tree_sitter_language::LanguageFn;", 1)
    s = s.replace("/// The enabled languages", f"{g}pub fn language_{n}() -> LanguageFn {{\n    {n}::{a.const}\n}}\n\n/// The enabled languages", 1)
    s = s.replace("    langs\n}", f'    {g.strip()}\n    langs.push(("{n}", language_{n}()));\n    langs\n}}', 1)
    return s


def regen(s):
    if a.crate is None or f'"{n}"' in s or f" {n} " in s:
        return s
    sub = f' "{a.subdir}"' if a.subdir else ""
    line = f'{a.mode} {n:<12}"$REG/{a.crate}"{sub}\n' if a.mode == "emit" else f'regen {n:<12}"$REG/{a.crate}"{sub}\n'
    return s.replace('echo "regenerated;', line + 'echo "regenerated;', 1)


def verify(s):
    if a.crate is None or re.search(rf"^\s*{n}\)", s, re.M):
        return s
    src = f"{a.subdir}/src" if a.subdir else "src"
    if a.mode == "emit":
        case = f'  {n})'.ljust(14) + f'CSRC="$REG/{a.crate}/{src}" ;;\n'
    else:
        case = f'  {n})'.ljust(14) + f'CSRC="$W/{n}/{src}" ;;\n'
    return s.replace('  *)          CSRC=', case + '  *)          CSRC=', 1)


edit("Cargo.toml", cargo)
edit("src/lib.rs", lib)
edit("tools/regen.sh", regen)
edit("tools/verify.sh", verify)

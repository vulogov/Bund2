#!/usr/bin/env bash
#
# scaffold-bund2.sh — create and prepare the Bund2 repository.
#
# Creates the workspace skeleton, the reference submodules used as the
# conformance oracle, the standing agent instructions, the RFC template,
# and the seeded decision and defect registers.
#
# Usage:
#   ./scaffold-bund2.sh [options]
#
# Options:
#   -d, --dir DIR       Target directory              (default: Bund2)
#   -n, --no-submodules Skip submodule fetch (offline)
#   -C, --no-commit     Do not create the initial commit
#   -f, --force         Write into a non-empty directory
#   -h, --help          Show this help
#
set -euo pipefail

TARGET="Bund2"
DO_SUBMODULES=1
DO_COMMIT=1
FORCE=0

GH="https://github.com/vulogov"
REFERENCE_REPOS=(
  "Bund"
  "bund_language_parser"
  "bundcore"
  "rust_multistackvm"
  "rust_multistack"
  "rust_dynamic"
)

# Internal crate dependency graph. Enforced at compile time from day one.
CRATES=(
  "bund2-value:"
  "bund2-api:bund2-value"
  "bund2-syntax:bund2-value"
  "bund2-ir:bund2-value,bund2-syntax"
  "bund2-interp:bund2-value,bund2-api,bund2-ir"
  "bund2-stdlib:bund2-value,bund2-api,bund2-ir"
  "bund2-jit:bund2-value,bund2-ir"
  "bund2-runtime:bund2-value,bund2-api,bund2-ir,bund2-interp,bund2-stdlib"
  "bund2-async:bund2-runtime"
  "bund2:bund2-runtime,bund2-syntax,bund2-ir,bund2-stdlib"
  "bund2-cli:bund2"
)

CRANELIFT_VERSION="0.135.0"
RUST_TOOLCHAIN="1.90.0"

say()  { printf '\033[1;34m==>\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33mwarn:\033[0m %s\n' "$*" >&2; }
die()  { printf '\033[1;31merror:\033[0m %s\n' "$*" >&2; exit 1; }

usage() { sed -n '3,20p' "$0" | sed 's/^# \{0,1\}//'; exit 0; }

while [[ $# -gt 0 ]]; do
  case "$1" in
    -d|--dir)           TARGET="${2:?--dir needs a value}"; shift 2 ;;
    -n|--no-submodules) DO_SUBMODULES=0; shift ;;
    -C|--no-commit)     DO_COMMIT=0; shift ;;
    -f|--force)         FORCE=1; shift ;;
    -h|--help)          usage ;;
    *)                  die "unknown option: $1 (try --help)" ;;
  esac
done

command -v git >/dev/null 2>&1 || die "git is required"
command -v cargo >/dev/null 2>&1 || warn "cargo not found; the workspace will not be verifiable here"

if [[ -e "$TARGET" ]]; then
  if [[ ! -d "$TARGET" ]]; then
    die "$TARGET exists and is not a directory"
  fi
  if [[ -n "$(ls -A "$TARGET" 2>/dev/null)" && $FORCE -eq 0 ]]; then
    die "$TARGET is not empty (use --force to write into it anyway)"
  fi
fi

mkdir -p "$TARGET"
cd "$TARGET"
ROOT="$PWD"
say "scaffolding into $ROOT"

# ---------------------------------------------------------------- git ------

if [[ ! -d .git ]]; then
  git init -q -b main
fi

# ------------------------------------------------------------ directories --

mkdir -p \
  .claude/commands \
  .github/workflows \
  crates \
  docs/research/grounding \
  docs/registers \
  docs/rfc/reviews \
  reference \
  tests/golden \
  xtask/src

# --------------------------------------------------------- reference repos --

if [[ $DO_SUBMODULES -eq 1 ]]; then
  say "adding reference submodules (analysis oracle, read-only)"
  for r in "${REFERENCE_REPOS[@]}"; do
    if [[ -d "reference/$r/.git" || -f "reference/$r/.git" ]]; then
      echo "    reference/$r already present"
      continue
    fi
    if git submodule add -q "$GH/$r" "reference/$r" 2>/dev/null; then
      echo "    reference/$r"
    else
      warn "could not add reference/$r — add it later with:"
      warn "  git submodule add $GH/$r reference/$r"
    fi
  done
  if git submodule status >/dev/null 2>&1; then
    git submodule status > reference/PINNED.txt || true
  fi
else
  say "skipping submodules (--no-submodules)"
  cat > reference/PINNED.txt <<'EOF'
Reference submodules not yet added. Run:

  for r in Bund bund_language_parser bundcore \
           rust_multistackvm rust_multistack rust_dynamic; do
    git submodule add https://github.com/vulogov/$r reference/$r
  done
  git submodule status > reference/PINNED.txt

The SHAs recorded here are what RFC citations resolve against. Bumping a
submodule invalidates every path:line citation made against the old SHA.
EOF
fi

# ------------------------------------------------------------- license -----

if [[ -f reference/Bund/LICENSE ]]; then
  cp reference/Bund/LICENSE LICENSE
  say "LICENSE copied from reference/Bund (Apache-2.0)"
else
  cat > LICENSE.TODO <<'EOF'
Apache License 2.0, matching the existing Bund repositories.

Fetch the canonical text from https://www.apache.org/licenses/LICENSE-2.0.txt
and save it as LICENSE, then delete this file.
EOF
  warn "LICENSE not copied; see LICENSE.TODO"
fi

# ------------------------------------------------------------ gitignore ----

cat > .gitignore <<'EOF'
/target
**/*.rs.bk
Cargo.lock.orig
.DS_Store

# Claude Code local settings (the shared file is committed)
.claude/settings.local.json

# Oracle build artifacts and scratch output
/target/oracle/
/tests/golden/.work/
EOF

# --------------------------------------------------------- rust toolchain --

cat > rust-toolchain.toml <<EOF
[toolchain]
channel = "$RUST_TOOLCHAIN"
components = ["rustfmt", "clippy"]
EOF

cat > rustfmt.toml <<'EOF'
edition = "2024"
max_width = 100
EOF

cat > deny.toml <<'EOF'
# cargo-deny: dependency discipline.
#
# This project prefers pure-Rust dependencies. Anything that pulls in a C or
# C++ toolchain at build time should be a deliberate, reviewed decision rather
# than something that arrives transitively.

[licenses]
allow = ["Apache-2.0", "MIT", "BSD-2-Clause", "BSD-3-Clause", "ISC", "Unicode-3.0", "Zlib"]

[bans]
multiple-versions = "warn"
wildcards = "deny"

[advisories]
yanked = "deny"

[sources]
unknown-registry = "deny"
unknown-git = "deny"
EOF

# --------------------------------------------------------- workspace root --

WS_MEMBERS=""
WS_INTERNAL_DEPS=""
for entry in "${CRATES[@]}"; do
  cname="${entry%%:*}"
  WS_MEMBERS+="$(printf '  "crates/%s",' "$cname")"$'\n'
  WS_INTERNAL_DEPS+="$(printf '%-14s = { path = "crates/%s", version = "0.0.0" }' \
                        "$cname" "$cname")"$'\n'
done

cat > Cargo.toml <<EOF
[workspace]
resolver = "3"

# The reference submodules are Cargo projects in their own right. Without this,
# Cargo auto-discovers this manifest as their parent workspace and refuses to
# build them. They must stay out: they are the conformance oracle, pinned by
# SHA, and editing their manifests would break the citations RFCs make against
# those SHAs.
exclude = ["reference"]

members = [
${WS_MEMBERS}  "xtask",
]

[workspace.package]
edition      = "2024"
version      = "0.0.0"
license      = "Apache-2.0"
repository   = "https://github.com/vulogov/Bund2"
rust-version = "$RUST_TOOLCHAIN"

[workspace.dependencies]
# Internal
${WS_INTERNAL_DEPS}
# Cranelift. Exact pins: cranelift-jit describes itself as experimental and
# the workspace moves on a roughly monthly cadence. Upgrades are deliberate.
cranelift-codegen  = "=$CRANELIFT_VERSION"
cranelift-frontend = "=$CRANELIFT_VERSION"
cranelift-module   = "=$CRANELIFT_VERSION"
cranelift-jit      = "=$CRANELIFT_VERSION"
cranelift-object   = "=$CRANELIFT_VERSION"

[profile.release]
lto           = "thin"
codegen-units = 1

[profile.bench]
debug = true
EOF

# ------------------------------------------------------------ crate stubs --

crate_doc() {
  case "$1" in
    bund2-value)   echo "BundValue: the runtime value representation. See RFC-0001." ;;
    bund2-api)     echo "Stable ABI for external Rust word packages. See RFC-0002." ;;
    bund2-syntax)  echo "Surface syntax: pest grammar to AST. See RFC-0003." ;;
    bund2-ir)      echo "BundIR: linear, span-carrying, effect-annotated IR. See RFC-0003." ;;
    bund2-interp)  echo "Tier 0: the BundIR interpreter. Mandatory on every target. See RFC-0003." ;;
    bund2-stdlib)  echo "The standard library: native words, effects, and JIT lowerings." ;;
    bund2-jit)     echo "Tier 1 and AOT: BundIR to CLIF. Feature-gated. See RFC-0005, RFC-0006." ;;
    bund2-runtime) echo "VM assembly: word slot table, tiering policy, execution context." ;;
    bund2-async)   echo "Executor integration and async native words. Feature-gated. See RFC-0007." ;;
    bund2)         echo "Library facade. The embedding API." ;;
    bund2-cli)     echo "The bund2 command line compiler and REPL." ;;
  esac
}

say "creating crate skeletons"
for entry in "${CRATES[@]}"; do
  name="${entry%%:*}"
  deps="${entry#*:}"
  dir="crates/$name"
  mkdir -p "$dir/src"

  {
    echo "[package]"
    echo "name = \"$name\""
    echo "description = \"$(crate_doc "$name")\""
    echo "version.workspace      = true"
    echo "edition.workspace      = true"
    echo "license.workspace      = true"
    echo "repository.workspace   = true"
    echo "rust-version.workspace = true"
    echo

    if [[ "$name" == "bund2-cli" ]]; then
      echo "[[bin]]"
      echo "name = \"bund2\""
      echo "path = \"src/main.rs\""
      echo
    fi

    echo "[dependencies]"
    if [[ -n "$deps" ]]; then
      IFS=',' read -ra ds <<< "$deps"
      for d in "${ds[@]}"; do
        echo "$d.workspace = true"
      done
    fi

    case "$name" in
      bund2-jit)
        echo "cranelift-codegen  = { workspace = true, optional = true }"
        echo "cranelift-frontend = { workspace = true, optional = true }"
        echo "cranelift-module   = { workspace = true, optional = true }"
        echo "cranelift-jit      = { workspace = true, optional = true }"
        echo "cranelift-object   = { workspace = true, optional = true }"
        echo
        echo "[features]"
        echo "default = []"
        echo "# AOT via cranelift-object. Build this before the JIT: object output"
        echo "# can be disassembled, diffed, and checked into a test corpus."
        echo "aot = [\"dep:cranelift-codegen\", \"dep:cranelift-frontend\", \"dep:cranelift-module\", \"dep:cranelift-object\"]"
        echo "jit = [\"dep:cranelift-codegen\", \"dep:cranelift-frontend\", \"dep:cranelift-module\", \"dep:cranelift-jit\"]"
        ;;
      bund2-runtime)
        echo "bund2-jit = { workspace = true, optional = true }"
        echo
        echo "[features]"
        echo "default = []"
        echo "# bund2-async depends on bund2-runtime, so it must NOT appear here."
        echo "aot = [\"dep:bund2-jit\", \"bund2-jit/aot\"]"
        echo "jit = [\"dep:bund2-jit\", \"bund2-jit/jit\"]"
        ;;
      bund2)
        echo "bund2-async = { workspace = true, optional = true }"
        echo
        echo "[features]"
        echo "default = []"
        echo "aot   = [\"bund2-runtime/aot\"]"
        echo "jit   = [\"bund2-runtime/jit\"]"
        echo "async = [\"dep:bund2-async\"]"
        ;;
      bund2-cli)
        echo
        echo "[features]"
        echo "default = []"
        echo "aot   = [\"bund2/aot\"]"
        echo "jit   = [\"bund2/jit\"]"
        echo "async = [\"bund2/async\"]"
        ;;
    esac
  } > "$dir/Cargo.toml"

  if [[ "$name" == "bund2-cli" ]]; then
    cat > "$dir/src/main.rs" <<EOF
//! $(crate_doc "$name")

fn main() {
    eprintln!("bund2: not yet implemented");
    std::process::exit(70);
}
EOF
  else
    cat > "$dir/src/lib.rs" <<EOF
//! $(crate_doc "$name")

#![forbid(unsafe_code)]
EOF
  fi
done

# bund2-jit and bund2-value will both need unsafe eventually; do not lie about it.
for c in bund2-jit bund2-value bund2-runtime; do
  if [[ -f "crates/$c/src/lib.rs" ]]; then
    sed -i.bak 's|^#!\[forbid(unsafe_code)\]$|#![deny(unsafe_op_in_unsafe_fn)]|' \
      "crates/$c/src/lib.rs" && rm -f "crates/$c/src/lib.rs.bak"
  fi
done

cat > crates/bund2-api/README.md <<'EOF'
# bund2-api

**This crate is the only stability guarantee in the workspace.**

External Rust word packages depend on this crate and nothing else from Bund2.
Every change to its public surface is a breaking change and needs a version
bump and a migration note, regardless of how small it looks.

Everything else in the workspace may churn freely. If you are tempted to widen
this crate so that something internal becomes reachable, widen the internal
crate instead and leave this one alone.

Specifically out of scope: Cranelift types. Exposing `FunctionBuilder` here
would pin every third-party package to an exact Cranelift version, which turns
the guarantee above into a false one. See RFC-0002.
EOF

# ------------------------------------------------------------------ xtask --

cat > xtask/Cargo.toml <<'EOF'
[package]
name        = "xtask"
description = "Project tooling: oracle capture, conformance, measurement."
publish     = false

version.workspace      = true
edition.workspace      = true
license.workspace      = true
rust-version.workspace = true

[dependencies]
EOF

cat > xtask/src/main.rs <<'EOF'
//! Project tooling. Run with `cargo xtask <command>`.
//!
//! Everything lives here rather than in shell scripts so it is cross-platform,
//! cargo-native, and type-checked by CI like the rest of the workspace.

const HELP: &str = "\
cargo xtask <command>

Oracle and conformance
  golden        Build reference/Bund and record expected final state for the
                hermetic example corpus into tests/golden/
  conform       Run Bund2 against the goldens; print N/M and fail on regression
  unblock       For each unimplemented word, count hermetic examples it alone
                gates; sort descending. This is the M6 work queue.

Evidence
  corpus        Scan the example corpus for uses of .id, .timestamp, bund.eval,
                load.lambdas, register, and post-construction LAMBDA mutation.
                Resolves decisions D1, D2, D3, D5, D12 from data.
  layout        Print size_of::<Value>() and allocation counts per operation
  arity         Probe every registered word against instrumented stacks and
                emit a first-cut stack-effect table (unblocks RFC-0004)
  bench         Criterion baseline over the corpus (Phase 0)
";

fn main() -> std::process::ExitCode {
    let cmd = std::env::args().nth(1).unwrap_or_default();
    match cmd.as_str() {
        "golden" | "conform" | "unblock" | "corpus" | "layout" | "arity" | "bench" => {
            eprintln!("xtask: `{cmd}` is not implemented yet");
            std::process::ExitCode::from(70)
        }
        "" | "-h" | "--help" | "help" => {
            print!("{HELP}");
            std::process::ExitCode::SUCCESS
        }
        other => {
            eprintln!("xtask: unknown command `{other}`\n");
            print!("{HELP}");
            std::process::ExitCode::FAILURE
        }
    }
}
EOF

mkdir -p .cargo
cat > .cargo/config.toml <<'EOF'
[alias]
xtask = "run --package xtask --"
EOF

# ------------------------------------------------------------- CLAUDE.md ---

cat > CLAUDE.md <<'EOF'
# Bund2

Re-implementation of the Bund concatenative language: multi-stack, dynamically
typed, metaprogramming, with a tiered execution model (BundIR interpreter, plus
an optional Cranelift JIT and AOT compiler). Bund syntax and logic are
preserved 100%.

Bund2 is not a refactor of Bund. It is a reimplementation against an oracle:
`reference/` holds the existing implementation, pinned, and `tests/golden/`
holds the state it produces for the example corpus.

## Attribution — absolute

Never add AI attribution anywhere, in any form. No `Co-Authored-By` trailers,
no "Generated with" lines, no AI credit in commit messages, PR bodies, RFCs,
code comments, or documentation. Authorship is the repository owner's alone.
This applies even where a settings key would otherwise inject it.

## Source grounding — required

Design claims must cite source. Every factual claim about existing behaviour
carries a `path:line` citation into `reference/`, and you must have read that
file in this session — never cite from memory or infer from a filename.

If a claim cannot be grounded, do not soften it into prose. Add it to
`docs/registers/open-questions.md` and mark the spot `[UNGROUNDED]`.

## reference/ is read-only

`reference/` holds pinned submodules of the existing implementation, for
analysis only. Never edit it and never treat a file there as a target of work.

Building it is the one exception, and only to produce goldens: that is what the
oracle is for. Always build it out-of-tree so the submodule stays clean —

    cargo build --release --manifest-path reference/Bund/Cargo.toml \
                --target-dir target/oracle

`git status` inside `reference/` must stay empty. A dirty submodule means a
`path:line` citation somewhere no longer resolves against the recorded SHA.

## tests/golden/ is sacred

A golden that disagrees with Bund2 is not the thing that changes. A failure has
exactly three dispositions: a Bund2 bug (fix Bund2), an original-implementation
bug (record in `docs/registers/defects.md`, then
`cargo xtask golden --accept <name> --reason <ref>`), or a deviation already
approved in the work item. An unplanned deviation is a decision: stop and take
it to `docs/registers/decisions.md` before changing code.

## Registers are append-only

`docs/registers/decisions.md` and `defects.md` are the shared state between
sessions. Add entries and change an entry's `status`; never delete or renumber.

Never adopt an OPEN decision's default silently. The defaults are for planning.
An RFC or work item that quietly takes one has made a language decision on the
owner's behalf. Stop and say which decision blocks you.

## Research documents are immutable

`docs/research/` is the reasoning trail. When an RFC contradicts one, record it
in `docs/research/ERRATA.md` — do not edit the original.

## Terminology

Tier 0 = the BundIR interpreter (mandatory, every target).
Tier 1 = the Cranelift JIT (optional). AOT = the cranelift-object build.
Word = a named callable. Slot = a word table entry. Workbench = the auxiliary
stack. Effect = a word's stack arity. Conformance = passing goldens over total.

## Health metric

`cargo xtask conform` prints N/M. That number is the project's status. The JIT
and AOT milestones must move it by exactly zero: they change speed, not
meaning, so any movement is a bug.
EOF

# -------------------------------------------------------- claude settings --

cat > .claude/settings.json <<'EOF'
{
  "$schema": "https://json.schemastore.org/claude-code-settings.json",
  "attribution": {
    "commit": "",
    "pr": "",
    "sessionUrl": false
  },
  "permissions": {
    "deny": [
      "Edit(./reference/**)",
      "Write(./reference/**)",
      "Edit(./tests/golden/**)",
      "Write(./tests/golden/**)"
    ]
  }
}
EOF

cat > .claude/commands/rfc-ground.md <<'EOF'
Read every file under $ARGUMENTS in `reference/`. Do not draft an RFC.

Produce a grounding note at `docs/research/grounding/<area>.md` containing:

- what each file does, one line each, with a `path:line` anchor
- every word or method registered, with its observed stack arity where the
  source makes it determinable
- every place a runtime-mutable table is read or written
- every place a value's identity, timestamp, tags, or attributes are read
- anything contradicting a claim in `docs/research/` — cite both sides
- anything you could not determine from the source

Then append new decisions to `docs/registers/decisions.md` and new defects to
`docs/registers/defects.md`.
EOF

cat > .claude/commands/rfc-draft.md <<'EOF'
Draft RFC-$ARGUMENTS.

1. Read `docs/research/05-rfc-roadmap.md` for this RFC's scope, dependencies,
   and the improvements assigned to it.
2. Read `docs/research/ERRATA.md`.
3. Read the grounding notes for this RFC's areas, then re-read the source they
   cite. Do not rely on the notes alone.
4. Read `docs/registers/decisions.md`. If an OPEN decision blocks this RFC,
   stop and say which. Do not take its default.
5. Draft to `docs/rfc/RFC-<n>-<slug>.md` from `docs/rfc/0000-template.md`.

Every behavioural claim gets a `path:line` citation. Mark ungrounded claims
`[UNGROUNDED]` and add them to `docs/registers/open-questions.md`. Record the
reference SHA you grounded against. Status starts as `Draft`.
EOF

cat > .claude/commands/rfc-review.md <<'EOF'
Review RFC-$ARGUMENTS as an adversarial reader. Do not edit the RFC.

- Verify every `path:line` citation by opening the file. Report any that do not
  say what the RFC claims. This is the main job — re-read the cited lines, not
  the RFC.
- Find claims that need a citation and lack one.
- Check each acceptance criterion is actually checkable, and name what checks
  it. "Faster" is not checkable.
- Check the Preservation analysis section enumerates every behaviour the design
  could change, including ones the author may not have noticed.
- Check consistency with accepted RFCs and with ERRATA.
- List what the RFC assumes but does not state.

Write findings to `docs/rfc/reviews/RFC-<n>-review-<date>.md`.
EOF

cat > .claude/commands/work-item.md <<'EOF'
Implement work item $ARGUMENTS.

Read the work item, the RFC section it cites, and the pinned reference source
it names. Then implement, with tests.

Rules:
- Behaviour listed under Preserve must match the reference exactly.
- Behaviour listed under Deviate is approved; anything else that differs is not.
  If you find yourself needing an unlisted deviation, stop and say so.
- Run `cargo xtask conform` before you finish. State the before and after
  numbers. No previously-green golden may regress.
- Never edit `tests/golden/`.
EOF

# ---------------------------------------------------------- RFC template ---

cat > docs/rfc/0000-template.md <<'EOF'
# RFC-NNNN: <title>

- Status: Draft | Proposed | Accepted | Superseded
- Depends on: RFC-xxxx
- Decisions consumed: Dn, Dn
- Reference SHA: <the reference/ commit this RFC was grounded against>
- Supersedes: <research document section, if any>

## Summary

One paragraph.

## Motivation

What is wrong today, with `path:line` citations.

## Current behaviour

What the existing implementation does. Every claim cited. This section is the
preservation contract — the design cannot be judged without it.

## Design

The proposal.

## Preservation analysis

For each behaviour in "Current behaviour": preserved exactly / preserved with a
stated deviation / deliberately changed. Every deviation needs explicit
sign-off and a decision register entry.

## Alternatives considered

Including the rejected ones, and why.

## Acceptance criteria

Checkable. "Faster" is not checkable. "Conformance unchanged at 114/114 and
>= 3x on benches/arith_loop.bund against the Phase 0 baseline" is.

## Open questions

Cross-referenced to the registers.
EOF

# ------------------------------------------------------------- registers ---

cat > docs/registers/decisions.md <<'EOF'
# Decision register

Append-only. Add entries, change `status`, never delete or renumber.

Status: OPEN | RESOLVED | SUPERSEDED. A default is for planning only — an RFC
or work item may never adopt one silently.

---

## D1 — `.id` format contract
Does `.id`'s exact nanoid format matter to existing programs, or is the
contract "unique opaque string"?
- Blocks: RFC-0001
- Default: lazy nanoid derived from counter plus VM seed (preserves format)
- Evidence: `cargo xtask corpus`
- Status: OPEN

## D2 — `.timestamp` precision
Is millisecond granularity the contract, or must two values constructed in
sequence differ?
- Blocks: RFC-0001
- Default: sampled clock at millisecond granularity
- Evidence: `cargo xtask corpus`
- Status: OPEN

## D3 — tier policy for `bund.eval` output
JIT-eligible, or permanently Tier 0? Permanently Tier 0 removes a whole class
of unbounded code-memory growth.
- Blocks: RFC-0003, RFC-0005
- Default: permanently Tier 0
- Status: OPEN

## D4 — integer width
Is full `i64` required, or are 51-bit integers acceptable? The latter allows
NaN-boxing and a smaller value.
- Blocks: RFC-0001
- Default: full `i64`
- Status: OPEN

## D5 — lambda body mutability
Can a LAMBDA body be mutated after construction, or is it write-once? Write-once
lets the compiled cache skip invalidation.
- Blocks: RFC-0003
- Default: assume mutable; invalidate on write
- Evidence: `cargo xtask corpus`
- Status: OPEN

## D6 — async granularity
Is fine-grained suspension inside a word required, or is VM-per-task enough?
- Blocks: RFC-0007
- Default: VM-per-task
- Status: OPEN

## D7 — concurrent VM count
Tens (actor model is fine) or thousands (per-VM word tables become the memory
story)?
- Blocks: RFC-0007
- Default: tens
- Status: OPEN

## D8 — existing external word packages
Do any Rust word packages exist outside this repository? If so, `bund2-api` is
a migration rather than a clean design.
- Blocks: RFC-0002
- Default: none; design freely
- Status: OPEN

## D9 — third-party CLIF lowerings
Should `Intrinsic` lowerings ever be exposed through `bund2-api`? Doing so pins
external packages to an exact Cranelift version.
- Blocks: RFC-0002
- Default: no
- Status: OPEN

## D10 — C toolchain requirement
May `bund2 build` require `cc`, or must the compiler be self-contained?
- Blocks: RFC-0006
- Default: yes, `cc`; `--emit=bundle` covers toolchain-free targets
- Status: OPEN

## D11 — external dependents of `compile_to_binary`
Does anything outside the project depend on the current bincode object format?
- Blocks: RFC-0003
- Default: no; version the IR format freshly
- Status: OPEN

## D12 — the `*` fold-family
Restrict the whole-stack variadic words in JIT-able positions, or accept them
as a permanent optimization barrier?
- Blocks: RFC-0004
- Default: accept as barrier
- Evidence: `cargo xtask corpus`
- Status: OPEN

## D13 — value semantics under Rc
Today `Value` is deep-cloned everywhere, so Bund has value semantics and cycles
are impossible by construction. Naive `Rc` would give reference semantics and
make cycles constructible. Confirm `Rc::make_mut` (clone-on-write) as the
correct preservation.
- Blocks: RFC-0001
- Default: yes, `Rc::make_mut`
- Status: OPEN
EOF

cat > docs/registers/defects.md <<'EOF'
# Defect register

Defects in the existing implementation, found during analysis. Each needs a
`disposition`: PRESERVE (Bund2 reproduces the bug) or FIX (Bund2 corrects it,
and the affected golden is regenerated with a reference to this entry).

Fixing a behavioural defect is a deviation from 100% preservation and needs an
explicit decision. Leaving `disposition` empty blocks any work item that would
touch the area.

---

## F1 — `unregister` registered twice
The class variant shadows the lambda variant, so lambda unregistration is
unreachable by name. The class one is presumably meant to be `unregister.class`.
- `reference/rust_multistackvm/src/stdlib/lambdas/registry.rs`
- Behavioural. Disposition:

## F2 — `if.false.in_workbench` uses the wrong stack
`stdlib_logic_if_false_in_workbench` passes `StackOps::FromStack`, not
`FromWorkBench`.
- `reference/rust_multistackvm/src/stdlib/logic/if_fun.rs`
- Behavioural. Disposition:

## F3 — `stdlib_math_op_inline` checks the wrong stack
The `FromWorkBench` arm checks `current_stack_len()` before separately checking
`workbench.len()`.
- `reference/rust_multistackvm/src/stdlib/math/math_op.rs`
- Behavioural. Disposition:

## F4 — redundant clone in `push_to_workbench`
Clones an owned value, pushes the clone, drops the original.
- `reference/rust_multistack/src/ts_workbench.rs`
- Performance only. Disposition: FIX

## F5 — `_inline` suffix rebuilt three times per call
`is_inline` formats it once; `get_inline` formats it again for `contains_key`
and a third time for `get`.
- `reference/rust_multistackvm/src/multistackvm_inline.rs`
- Performance only. Disposition: FIX

## F6 — alias resolved twice per CALL
Once in `apply`, again in `i()`.
- `reference/rust_multistackvm/src/multistackvm_apply.rs`, `multistackvm_inline.rs`
- Performance only. Disposition: FIX

## F7 — instrumentation in the dispatch path
`time_graph::instrument` on `apply`, `i`, `i_direct`, `call`, `lambda_eval`,
`stdlib_execute_base_inline`, `stdlib_logic_if_base`, `stdlib_logic_times`.
Must be removed or feature-gated before any baseline measurement.
- Performance only. Disposition: FIX

## F8 — unbounded inter-crate version pins
`">=0.*.*"` between the five library crates: a `Value` layout change propagates
silently.
- Resolved by the monorepo. Disposition: FIX (structural)

## F9 — the parser has a side channel
The `ctx` rule mutates the caller's `state` vector rather than returning a
subtree, which makes `( ... )` unanalysable.
- `reference/bund_language_parser/src/vm/ctx.rs`
- Structural. Disposition: FIX (scoped block node, RFC-0003)

## F10 — debugger history written to the working directory
- `reference/Bund/src/stdlib/functions/debug_fun/`
- Cosmetic. Disposition: FIX

## F11 — inverted guard in `register_method_value_init`
`if ! value.type_of() == OBJECT` parses as `(!value.type_of()) == OBJECT`; the
guard never fires as intended.
- `reference/Bund/src/stdlib/functions/oop/value_class.rs`
- Behavioural. Disposition:
EOF

cat > docs/registers/open-questions.md <<'EOF'
# Open questions

Claims that could not be grounded in `reference/`, and questions raised during
drafting that are not yet decisions.

An entry here is a `[UNGROUNDED]` marker in some RFC. Either ground it, or
promote it to a decision, or delete the claim.

| # | Question | Raised in | Status |
|---|----------|-----------|--------|
EOF

# ------------------------------------------------------- research folder ---

cat > docs/research/README.md <<'EOF'
# Research

The reasoning trail that preceded the RFCs. **These documents are immutable.**

Expected contents, in order:

    00-jit-feasibility.md
    01-extensibility-async.md
    02-native-binaries.md
    03-metaprogramming-oop-debugger.md
    04-consolidated-architecture.md
    05-rfc-roadmap.md
    ERRATA.md

When an RFC contradicts one of these, record the supersession in `ERRATA.md`
rather than editing the original. They contain reasoning that was correct given
what was known at the time, and the trail of what changed and why is worth more
than a tidy current-state document.

`grounding/` holds the per-area source notes produced by `/rfc-ground`.
EOF

cat > docs/research/ERRATA.md <<'EOF'
# Errata

Supersessions of `docs/research/`. One line each. Never edit the originals.

Format: `<document> §<section> -> superseded by <RFC> §<section> (<reason>)`

---

- `00-jit-feasibility.md` §4.1 -> superseded by the OOP finding in
  `03-metaprogramming-oop-debugger.md` §2.2: `.id` and `.timestamp` are public
  API on the base `Object` class, so the value cannot shrink to 16 bytes.

- `02-native-binaries.md` §4 -> superseded by `03-metaprogramming-oop-debugger.md`
  §0.2: `bund.eval` exists, so closed-world analysis rarely fires and the front
  end is mandatory in every binary.
EOF

# ---------------------------------------------------------- golden README --

cat > tests/golden/README.md <<'EOF'
# Golden outputs

Expected final state for the hermetic example corpus, captured from the
reference implementation in `reference/Bund`.

Of the 120 example programs and 12 test programs in the reference repository,
those that touch no network, filesystem, image, database, or bus form the
hermetic conformance suite. The remainder are manual smoke tests.

Each golden records the full final state, not stdout: every named stack's
contents, the workbench, the exit status, the error text, and stdout. A
concatenative program's meaning is what it leaves on the stacks.

## These files are sacred

When Bund2 disagrees with a golden, **the golden is not the thing that
changes**. A failure has exactly three dispositions:

1. **Bund2 bug** — fix Bund2. This is the common case.
2. **Reference bug** — record it in `docs/registers/defects.md` with a
   disposition, then regenerate that one golden:
   `cargo xtask golden --accept <name> --reason F<n>`
   and note it in `EXCEPTIONS.md`.
3. **Approved deviation** — already listed in the work item's Deviate section.
   If it is not listed, stop: an unplanned deviation is a decision, and it goes
   to `docs/registers/decisions.md` before any code changes.

Quietly regenerating a golden because it is easier is not a fourth option. CI
treats this directory as read-only; regeneration goes through `--accept` with a
reason.

Regenerate everything only when a reference submodule SHA changes, and review
the diff as carefully as you would review code.
EOF

cat > tests/golden/EXCEPTIONS.md <<'EOF'
# Golden exceptions

Goldens deliberately regenerated because the reference implementation has a
defect Bund2 does not reproduce.

| Golden | Defect | Date | Reason |
|--------|--------|------|--------|
EOF

# ----------------------------------------------------------------- CI ------

cat > .github/workflows/ci.yml <<'EOF'
name: ci

on:
  push:
    branches: [main]
  pull_request:

env:
  CARGO_TERM_COLOR: always
  RUSTFLAGS: -D warnings

jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          submodules: recursive
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy
      - run: cargo fmt --all -- --check
      - run: cargo clippy --workspace --all-targets
      - run: cargo test --workspace
      - name: feature matrix
        run: |
          cargo check --workspace --features aot
          cargo check --workspace --features jit
          cargo check --workspace --features async

  conformance:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          submodules: recursive
      - uses: dtolnay/rust-toolchain@stable
      - name: conformance against the oracle
        run: cargo xtask conform
      - name: goldens unmodified
        run: git diff --exit-code -- tests/golden/

  deny:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: EmbarkStudios/cargo-deny-action@v2
EOF

# ------------------------------------------------------------- README ------

cat > README.md <<'EOF'
# Bund2

A JIT-capable reimplementation of the Bund concatenative language: multi-stack,
dynamically typed, metaprogramming, with a tiered execution model.

Bund syntax and logic are preserved 100%.

## Status

Pre-implementation. The RFC series is being drafted; see `docs/rfc/` and
`docs/research/05-rfc-roadmap.md`.

## Layout

    crates/         the Bund2 workspace
    docs/research/  the analysis that preceded the RFCs (immutable)
    docs/rfc/       the RFC series
    docs/registers/ decision and defect registers
    reference/      the existing implementation, pinned — analysis only
    tests/golden/   expected state captured from the reference implementation
    xtask/          project tooling

## Execution tiers

    Tier 0   BundIR interpreter          mandatory, every target
    Tier 1   Cranelift JIT               optional, feature = "jit"
    AOT      cranelift-object -> native  optional, feature = "aot"

## Health metric

    cargo xtask conform

Prints passing goldens over total. That number is the project's status. The JIT
and AOT milestones must move it by exactly zero.

## Building

    cargo build --workspace
    cargo build --workspace --features jit

## License

Apache-2.0.
EOF

# --------------------------------------------------------------- commit ----

say "scaffold written"

if [[ $DO_COMMIT -eq 1 ]]; then
  if ! git config user.email >/dev/null 2>&1 || ! git config user.name >/dev/null 2>&1; then
    warn "git user.name / user.email not configured; skipping initial commit"
    DO_COMMIT=0
  fi
fi

if [[ $DO_COMMIT -eq 1 ]]; then
  git add -A
  git commit -q --no-verify -m "Scaffold Bund2 workspace, reference oracle, and RFC process

Workspace skeleton for the eleven crates, with the internal dependency graph
wired so layering violations are compile errors. Cranelift pinned exactly.

Reference implementations added as pinned submodules for use as a conformance
oracle. Decision and defect registers seeded from the analysis phase."
  say "initial commit created"
else
  say "no commit created; review and commit when ready"
fi

# --------------------------------------------------------------- next ------

cat <<'EOF'

Next steps
----------

1. Copy the six research documents into docs/research/ using the names in
   docs/research/README.md.

2. Verify the workspace builds:

       cargo check --workspace

3. Build the oracle. This is the first real task and everything depends on it:

       cargo xtask golden

   Implement it to build reference/Bund, run each hermetic example, and record
   the full final state — every named stack, the workbench, exit status, error
   text, stdout — into tests/golden/.

4. Run the evidence tools. `corpus` is the cheapest high-value session in the
   project: it resolves decisions D1, D2, D3, D5 and D12 from counted evidence
   rather than judgement.

       cargo xtask corpus
       cargo xtask layout
       cargo xtask bench

5. Then draft RFC-0000 and RFC-0001. RFC-0001 is accepted against the numbers
   from step 4, not before.

Confirm attribution is off after your first agent-authored commit:

       git log -1 --format=%B

EOF

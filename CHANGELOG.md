# Changelog

Every entry records what changed and whether it breaks a program that worked before, per
[docs/RELEASES.md](docs/RELEASES.md). Nothing has been tagged yet.

## Unreleased

### Added

- **A WebAssembly target.** `dovik build file.dovk --target wasm` writes a `wasm32-wasip1`
  module that runs under any WASI host. Needs `wasi-libc` when the compiler is built; without
  it the native build is unaffected and a wasm build fails with that message. Design and the
  one limitation: [docs/PROPOSAL-WASM.md](docs/PROPOSAL-WASM.md).
- **`--depth-guard`**, which counts call depth instead of relying on a signal handler to catch
  stack exhaustion, in the functions that can reach themselves. A wasm build turns it on by
  itself; a native build emits nothing. [docs/DECISIONS.md](docs/DECISIONS.md) D030.
- **Program arguments under `dovik run`**, after `--`: `dovik run fib.dovk -- 10`.
- **A sanitizer suite and a frontend fuzzer**, both of which fail when the bug they look for is
  reintroduced. `bench/run.py` also measures the wasm target now
  ([docs/BENCHMARKS.md](docs/BENCHMARKS.md) section 12).
- `DOVIK_ALLOC_LIMIT`, which makes out-of-memory reachable on purpose, and `DOVIK_MAX_DEPTH`,
  which sets the depth-guard limit. Both are test hooks, not interface (RELEASES 2).

### Fixed

- **The parser looped forever on a failed condition before an `else`.** Five lines were enough
  to hang `dovik check`. Statement recovery now proves it consumed a token.
- **`dovik format` deleted `none`.** The token table mapped anything it did not list to the
  empty string, so `--write` rewrote `if x == none:` into `if x == :` — which did not parse,
  and which the parser then looped on. The table is exhaustive now, so a new token is a compile
  error there instead of silent deletion. `examples/wordcount.dovk` had been committed in the
  rewritten state.
- `Int?` formatted as `Int ?`.
- A second file path on the command line silently replaced the first instead of being an error.

### Changed

- Nothing breaking. Every change above either adds a flag, fixes a defect, or touches a
  documented test hook.

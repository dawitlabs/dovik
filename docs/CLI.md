# Dovik Command-Line Interface

**Status:** Implemented, including multi-file programs

Command names and behavior remain proposals rather than a contract; the only contract is [SPEC.md](SPEC.md). Section 8 records what the current CLI actually does.

## 1. Purpose

The `dovik` CLI is the primary interface for compiling, checking, running, and testing Dovik programs.

It should be simple for developers, predictable for scripts, and reliable for AI coding agents.

## 2. Proposed Commands

### `dovik build`

Compile a Dovik program or project into an executable.

### `dovik run`

Compile and execute a Dovik program. This compiles first and then runs; Dovik has no interpreter or REPL ([DECISIONS.md](DECISIONS.md) D016).

Arguments for the program go after `--`: `dovik run fib.dovk -- 10`. Everything before `--` is read by `dovik`, everything after is read by the program through `argument_count` and `argument` ([STDLIB.md](STDLIB.md) 6).

### `dovik check`

Check syntax, names, and types without producing a final executable.

### `dovik test`

Run the project's automated tests.

### `dovik format`

Format source files consistently.

### `dovik --version`

Print the installed compiler version.

### `dovik --help`

Display available commands and usage.

## 3. Diagnostics

Compiler errors should include:

- A stable error code where appropriate.
- The source file and location.
- A concise explanation.
- A suggested correction when practical.

## 4. Automation and AI-Agent Support

The CLI should provide:

- Reliable exit codes.
- Deterministic behavior where practical.
- Machine-readable diagnostic output, including JSON.
- Test results that can be parsed by other tools.
- Clear separation between diagnostics and ordinary program output.

## 5. Project Structure

Every command takes one `.dovk` file. That file is the entry module, and the compiler
discovers the rest of the program by following its imports transitively
([SPEC.md](SPEC.md) 22.7). There is no manifest, no search path, and nothing to configure:
an import path is relative to the file that writes it.

`dovik test <file>` collects tests from that file only, not from the modules it imports. A
test file is therefore a file that imports the module under test and holds nothing but
tests, which is how [PROPOSAL-MODULES.md](PROPOSAL-MODULES.md) answers section 6's question
about where tests belong.

A project manifest and dependency management remain unspecified and belong after the module
system rather than inside it: a manifest describes a project, and nothing in the language
needs one to compile a program.

## 6. Open Decisions

- Project manifest format, and dependency management. Both are deliberately absent rather than
  pending; see [DECISIONS.md](DECISIONS.md) D029 for the measurement and the trigger that would
  change it.
- Incremental compilation. Also absent by measurement rather than by omission: on a 4,000-line
  program across nine modules, `dovik check` takes 85 ms and a full `dovik build` takes 495 ms,
  so the whole frontend is 17% of a build and the rest is `clang` with full link-time
  optimization, which is whole-program by design (D029).
- Whether the `test_` prefix is the right convention now that a test file can be a module of
  its own.
- Whether `dovik test` should collect tests from the whole import graph rather than the entry
  file ([SPEC.md](SPEC.md) 22.7).

Settled provisionally by the current implementation, and open to change:

- Default executable naming: the source file's stem, written to the working directory.
- Build output directory: none; the executable is written where it is named.
- JSON diagnostic schema: see section 8.

## 7. Status

Every command in section 2 is implemented.

## 8. Implemented Behaviour

`dovik test <file>` collects every module-level function whose name begins with `test_`
and which takes no parameters, then runs each one in a separate process so that a failure
does not stop the run. A test fails by raising a runtime error, which `assert` does. The
runner prints one line per test and a summary, and exits 1 if any failed.

This needed no language change. A naming convention plus `assert` was enough, which is why
there is no `test` keyword and no attribute syntax.


`dovik build <file> [-o <path>]` writes an executable, defaulting to the file's
stem. `--emit-llvm`, `--emit-ir` and `--emit-tokens` write that stage to stdout
instead. `--no-opt` skips optimization.

`dovik run <file> [-- <args>]` builds to a temporary path, executes it, forwards
its exit code, and removes it. It does not interpret
([DECISIONS.md](DECISIONS.md) D016). `--` ends `dovik`'s own options, so a
program may be passed a flag of its own: `dovik run f.dovk -- --json` gives the
program `--json`.

A second file path is an error rather than a replacement for the first. Only one
entry file is ever compiled, and the rest of a program is found by following its
imports ([SPEC.md](SPEC.md) 22).

`dovik check <file>` runs every pass and reports diagnostics without generating
code. It prints nothing and exits 0 when the program is valid.

Exit codes are 0 on success and 1 on any failure, including a runtime error in a
program started by `dovik run`.

Diagnostics go to stderr; program output goes to stdout. Nothing a compiler pass
produces is ever written to stdout.

Every diagnostic carries a stable code ([SPEC.md](SPEC.md) 15.3), prefixed by the
pass that raised it: `DVK1xxx` lexical, `DVK2xxx` syntax, `DVK3xxx` names and
scope, `DVK4xxx` types, `DVK5xxx` runtime. A code, once assigned, is never reused
for a different error.

Text form:

```
main.dovk:4:17: error[DVK4001]: annotation says `Float` but the initializer is `Int`
```

`--json` emits one object per line instead:

```json
{"severity":"error","code":"DVK4001","file":"main.dovk","line":4,"column":17,"start":37,"end":38,"message":"..."}
```

`start` and `end` are byte offsets into the source. A runtime error is reported by
the program itself, in the form `runtime error[DVK5005] at 3:7: index out of bounds`.

# Dovik Command-Line Interface

**Status:** Draft — no CLI exists

Nothing in this document is implemented. Command names and behavior are proposals, not a contract; the only contract is [SPEC.md](SPEC.md).

## 1. Purpose

The `dovik` CLI is the primary interface for compiling, checking, running, and testing Dovik programs.

It should be simple for developers, predictable for scripts, and reliable for AI coding agents.

## 2. Proposed Commands

### `dovik build`

Compile a Dovik program or project into an executable.

### `dovik run`

Compile and execute a Dovik program. This compiles first and then runs; Dovik has no interpreter or REPL ([DECISIONS.md](DECISIONS.md) D016).

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

The initial version compiles a single `.dovk` file. Dovik has no module system ([SPEC.md](SPEC.md) section 16), so multi-file programs are not expressible yet, and a manifest would have nothing to describe.

A project manifest and dependency management belong with the module system, in Roadmap Phase 9.

## 6. Open Decisions

- Exact command syntax and flags.
- Default executable naming.
- JSON diagnostic schema.
- Build output directory.
- Project manifest format.
- Dependency management.
- Incremental compilation and caching.

## 7. Status

This document defines the proposed CLI, not a finalized interface contract. `dovik build`, `run`, `check`, `test`, and `format` are all unimplemented.

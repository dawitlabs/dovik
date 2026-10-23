# Dovik Development Roadmap

**Status:** Phases 1 through 5, 7 and 9 are complete; Phase 6 is implemented but incomplete. See **Status** at the end.

Phases were gated by the open language questions in [SPEC.md](SPEC.md) section 21. **All of them are now resolved** ([DECISIONS.md](DECISIONS.md) D013, D019, D020, D022, D023, D025, D026, D027), so no phase is blocked on a language question any more. What remains is implementation.

## Phase 1: Define the Language

- Establish the language vision and core specification. *(done)*
- Define variables, types, functions, control flow, and collections. *(done — [SPEC.md](SPEC.md))*
- Document compiler architecture, runtime requirements, CLI, and platform goals. *(done)*
- Ratify D011 (shadowing over globals) and D013 (reference semantics for collections). *(done — both Accepted)*
- Resolve U6 (mutual recursion) and U2 (division by zero). *(done — [DECISIONS.md](DECISIONS.md) D019 and D020)*

**Completion criterion:** The specification is unambiguous enough that two people would implement the same language from it, every decision Phase 2 or Phase 3 depends on has been ratified, and nothing blocking those phases is still open. **This criterion is now met.**

**Blocked by:** nothing.

**Nothing is open.** Section 21 is empty.

## Phase 2: Compiler Foundation

- Set up the Rust project and development workflow. *(done)*
- Implement a lexer and parser. *(done)*
- Build an abstract syntax tree (AST). *(done)*
- Report syntax errors with source locations. *(done)*
- Add automated tests for valid and invalid source code. *(done)*

**Completion criterion:** Dovik can parse source files and report useful syntax errors, including bad indentation, tab indentation, empty blocks, and a missing block-header colon. **Met.** All four named cases have a test: `tabs_in_indentation_are_rejected`, `indentation_must_be_a_multiple_of_four` and `indentation_may_not_skip_a_level` in `tests/lexer.rs`, and `block_must_be_indented` and `block_header_must_carry_its_colon` in `tests/parser.rs`.

## Phase 3: Type System

- Collect module-level function signatures before checking any body, so a function can call one declared later and mutual recursion works ([SPEC.md](SPEC.md) 5.5, [DECISIONS.md](DECISIONS.md) D019). *(done)*
- Implement name resolution and the scope rules in [SPEC.md](SPEC.md) section 9, including shadowing over globals (9.3) and local bindings (9.6). *(done)*
- Implement static type checking and type inference, including the value/reference split ([SPEC.md](SPEC.md) 3.4). *(done)*
- Identify recursive cycles in the call graph and require an explicit return annotation where the cycle makes inference circular. *(done)*
- Reject a module-scope call that can reach a function reading a module-scope variable declared after it ([SPEC.md](SPEC.md) 8.5). *(done)*
- Validate function calls, assignments, and return values. *(done)*
- Produce clear diagnostics for type errors. *(done)*

**Completion criterion:** Invalid programs are rejected before code generation, every compile-time error in [SPEC.md](SPEC.md) 15.1 has a test, and mutual recursion both works and is diagnosed when it needs an annotation. **Met.** The compiler carries 88 error kinds across the lexer, parser, resolver, type checker and module loader; 87 are exercised by a test, by variant or by the diagnostic the test asserts. The exception is `OutOfMemory`, which is a runtime error (15.2) and has no deterministic trigger.

## Phase 4: First Native Executables

- Introduce an intermediate representation (IR). *(done)*
- Generate LLVM IR. *(done)*
- Produce native executables for Linux x86-64. *(done)*
- Support basic integer arithmetic, variables, and control flow. *(done)*
- Automate compilation and execution tests. *(done)*

The backend emits LLVM IR as text and links it with `clang` at `-O2 -flto=full`; the decisions [ARCHITECTURE.md](ARCHITECTURE.md) section 7 left open were settled that way, except the choice of Rust bindings, which turned out not to be needed.

**Completion criterion:** A Dovik program can compile and run as a native executable, and the checked-arithmetic errors hold at every optimization level — integer overflow ([SPEC.md](SPEC.md) 7.2) and division by zero ([SPEC.md](SPEC.md) 6.2). **Met.**

**Blocked by:** nothing in section 21, but it depends on the implementation decisions in [ARCHITECTURE.md](ARCHITECTURE.md) section 7 — LLVM version, bindings, lowering strategy, linking — which should be settled before this phase rather than during it.

## Phase 5: Core Language

- Implement functions, including direct and mutual recursion. *(done)*
- Implement strings, collections, and essential operators. *(done)*
- Establish consistent runtime behavior. *(done — the interpreter and the native build are tested against each other)*
- Improve error messages and test coverage. *(done)*

**Completion criterion:** Dovik supports its documented core language features, with tests for the runtime errors in [SPEC.md](SPEC.md) 15.2 — including division by zero for both `Int` and `Float` operands. **Met for nine of the ten runtime errors.** `OutOfMemory` is implemented and reachable but has no test, because provoking it deterministically means constraining the allocator, which the runtime does not yet allow.

**Blocked by:** nothing. U5 is resolved ([DECISIONS.md](DECISIONS.md) D027): a function may mutate a reference-typed global, which is shared mutable state without new syntax.

## Phase 6: Runtime and Standard Library

- Implement memory management. *(partly — reference counting, which does not reclaim cycles)*
- Add console and file I/O. *(done)*
- Implement essential collection and string operations. *(done)*
- Establish a stable runtime interface. *(done — `compiler/runtime-c/`)*
- Measure runtime overhead and memory usage. *(done — [BENCHMARKS.md](BENCHMARKS.md))*

**Completion criterion:** Ordinary programs can use core features without manual memory management, and cyclic garbage is reclaimable ([SPEC.md](SPEC.md) section 14). **Not met.** The first half holds. The second does not: reference counting leaks a cycle, and nothing collects one. This is the one place where the implementation contradicts the specification, which promises in 14 that unreachable values *including those in reference cycles* are eventually reclaimable. Either a cycle collector is built or 14 is amended; leaving both as they are is not an option.

**Blocked by:** nothing in section 21; U3, U7 and U8 are all resolved ([DECISIONS.md](DECISIONS.md) D026, D022, D025). The memory-management algorithm is also still unchosen ([RUNTIME.md](RUNTIME.md) section 8) and belongs before this phase, not inside it — and it must handle objects reachable from several bindings at once, since collections alias by default ([DECISIONS.md](DECISIONS.md) D013).

## Phase 7: Developer Experience

- Complete the initial `dovik` CLI. *(done — `build`, `run`, `check`, `format`, `test`)*
- Support building, running, checking, and testing programs. *(done)*
- Add consistent source formatting. *(done)*
- Improve diagnostics and machine-readable output. *(done — stable codes and `--json`)*
- Document installation and basic usage. *(done — [../README.md](../README.md))*

**Completion criterion:** Developers can create, compile, debug, and test small single-file programs reliably. Multi-file projects arrive in Phase 9. **Met.**

**Still open here:** the two `dovik test` questions in [CLI.md](CLI.md) section 6 — whether `test_` is the right convention now that a test file can be a module, and whether `dovik test` should collect from the whole import graph ([SPEC.md](SPEC.md) 22.7). The JSON schema is settled (CLI.md section 8).

## Phase 8: Performance and Stability

- Establish repeatable benchmarks. *(done — `bench/run.py`, results with their environment)*
- Profile compiler and runtime performance. *(done)*
- Optimize based on measured bottlenecks. *(done — and two optimizations were measured and declined)*
- Test memory safety and runtime correctness. *(partly — the test suite covers correctness; no sanitizer or fuzzing run exists)*
- Establish release and compatibility practices. *(not started)*

**Completion criterion:** Performance claims are supported by reproducible results published with their environment and configuration ([BENCHMARKS.md](BENCHMARKS.md)). Until this phase completes, Dovik publishes no performance numbers at all.

**Blocked by:** nothing in section 21. Reclamation-pause measurement depends on the memory strategy chosen in Phase 6.

## Phase 9: Modules and Larger Projects

- Define module and import semantics. *(done — [SPEC.md](SPEC.md) 22, [DECISIONS.md](DECISIONS.md) D021)*
- Support larger multi-file programs. *(done — the entry file's import graph compiles as one program)*
- Introduce project configuration. *(not needed yet — [DECISIONS.md](DECISIONS.md) D029)*
- Establish dependency management. *(waits for a program that wants code it did not write — D029)*
- Improve incremental compilation where practical. *(measured and declined — the frontend is 17% of a build — D029)*

**Completion criterion:** Dovik can support maintainable multi-module applications.

**Blocked by:** nothing. The module system is specified in [SPEC.md](SPEC.md) 22 and implemented; [PROPOSAL-MODULES.md](PROPOSAL-MODULES.md) is the design record. Project configuration and dependency management remain, and neither is blocked by a language question.

## Phase 10: Platform Expansion

- Evaluate additional processor architectures and operating systems.
- Add Windows and macOS support.
- Investigate WebAssembly.
- Design mobile platform integrations.
- Validate portability of the runtime and standard library.

**Completion criterion:** Each new target has working builds, tests, and documented limitations. No document may describe a target as supported before that point ([PLATFORMS.md](PLATFORMS.md)).

**Blocked by:** cross-compilation strategy ([ARCHITECTURE.md](ARCHITECTURE.md) section 7) and runtime portability. Mobile and WebAssembly each need design work that has not started.

## Phase 11: Self-Hosting

- Implement enough of Dovik to express substantial compiler functionality.
- Develop a Dovik implementation of the compiler.
- Bootstrap it using the existing Rust-based compiler.
- Verify that the resulting compiler can rebuild itself.
- Maintain reproducible bootstrap tests.

**Completion criterion:** Dovik can compile its compiler without relying on the Rust implementation for the final bootstrap stage.

**Blocked by:** language features a compiler needs that Dovik deliberately lacks, user-defined types chief among them. Modules (Phase 9) and a way to represent absence ([SPEC.md](SPEC.md) 23) both exist now. Each remaining one must earn its way in under [SPEC.md](SPEC.md) section 20 before this phase is realistic.

## Development Principles

- Prioritize correctness over premature optimization.
- Implement one small, testable feature at a time.
- Keep the specification synchronized with actual behavior.
- Avoid implementing future platform features before the foundation is stable.
- Make architectural decisions explicit and revise them when evidence justifies it.
- Keep the compiler understandable and maintainable.

## Status

This roadmap is provisional. Phases may overlap or change as technical constraints become clearer. Completion is determined by working implementations and tests, not by documentation alone.

Current position: **Phases 1 through 5, 7 and 9 are complete. Phase 6 is implemented but its criterion is not met, Phase 8 is partly done, and Phases 10 and 11 have not started.**

Dovik programs compile to native Linux x86-64 executables and run. The frontend, type checker,
IR, LLVM backend, C runtime, standard library increments, module system, optional types and the
`dovik` CLI all exist, under 389 tests. Every language question [SPEC.md](SPEC.md) section 21
raised is resolved, so nothing is blocked on a decision.

What is actually left:

- **Phase 6** is incomplete on one point: reference counting does not reclaim reference cycles,
  which its completion criterion requires — and which [SPEC.md](SPEC.md) section 14 promises.
- **Phase 8** has repeatable benchmarks and recorded results ([BENCHMARKS.md](BENCHMARKS.md));
  the release and compatibility practices it also asks for are not established, and the memory
  safety it asks to test has no sanitizer or fuzzing run behind it.
- **One runtime error has no test.** `OutOfMemory` is implemented and reachable, but provoking
  it deterministically needs an allocator limit the runtime does not expose. It is the single
  gap in [SPEC.md](SPEC.md) 15.1 and 15.2 coverage; the other 87 error kinds are tested.
- **Phase 9** keeps project configuration and dependency management open, both declined for now
  ([DECISIONS.md](DECISIONS.md) D029).
- **Phase 10** has not started. Linux x86-64 is the only target.
- **Phase 11** waits on language features Dovik does not have, user-defined types chief among them.

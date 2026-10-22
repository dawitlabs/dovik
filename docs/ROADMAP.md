# Dovik Development Roadmap

**Status:** Draft — Phase 1's blocking decisions are settled; no later phase has started.

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

- Set up the Rust project and development workflow.
- Implement a lexer and parser.
- Build an abstract syntax tree (AST).
- Report syntax errors with source locations.
- Add automated tests for valid and invalid source code.

**Completion criterion:** Dovik can parse source files and report useful syntax errors, including bad indentation, tab indentation, empty blocks, and a missing block-header colon.

**Blocked by:** nothing. The lexical and block rules ([SPEC.md](SPEC.md) sections 2.3 through 2.8) are settled, and Phase 1's ratifications are done. This phase can start.

## Phase 3: Type System

- Collect module-level function signatures before checking any body, so a function can call one declared later and mutual recursion works ([SPEC.md](SPEC.md) 5.5, [DECISIONS.md](DECISIONS.md) D019).
- Implement name resolution and the scope rules in [SPEC.md](SPEC.md) section 9, including shadowing over globals (9.3) and local bindings (9.6).
- Implement static type checking and type inference, including the value/reference split ([SPEC.md](SPEC.md) 3.4).
- Identify recursive cycles in the call graph and require an explicit return annotation where the cycle makes inference circular.
- Reject a module-scope call that can reach a function reading a module-scope variable declared after it ([SPEC.md](SPEC.md) 8.5).
- Validate function calls, assignments, and return values.
- Produce clear diagnostics for type errors.

**Completion criterion:** Invalid programs are rejected before code generation, every compile-time error in [SPEC.md](SPEC.md) 15.1 has a test, and mutual recursion both works and is diagnosed when it needs an annotation.

**Blocked by:** nothing. D011, D013, and D019 are all Accepted, and they are precisely what this phase implements.

## Phase 4: First Native Executables

- Introduce an intermediate representation (IR).
- Generate LLVM IR.
- Produce native executables for Linux x86-64.
- Support basic integer arithmetic, variables, and control flow.
- Automate compilation and execution tests.

**Completion criterion:** A Dovik program can compile and run as a native executable, and the checked-arithmetic errors hold at every optimization level — integer overflow ([SPEC.md](SPEC.md) 7.2) and division by zero ([SPEC.md](SPEC.md) 6.2).

**Blocked by:** nothing in section 21, but it depends on the implementation decisions in [ARCHITECTURE.md](ARCHITECTURE.md) section 7 — LLVM version, bindings, lowering strategy, linking — which should be settled before this phase rather than during it.

## Phase 5: Core Language

- Implement functions, including direct and mutual recursion.
- Implement strings, collections, and essential operators.
- Establish consistent runtime behavior.
- Improve error messages and test coverage.

**Completion criterion:** Dovik supports its documented core language features, with tests for the runtime errors in [SPEC.md](SPEC.md) 15.2 — including division by zero for both `Int` and `Float` operands.

**Blocked by:** nothing. U5 is resolved ([DECISIONS.md](DECISIONS.md) D027): a function may mutate a reference-typed global, which is shared mutable state without new syntax.

## Phase 6: Runtime and Standard Library

- Implement memory management.
- Add console and file I/O.
- Implement essential collection and string operations.
- Establish a stable runtime interface.
- Measure runtime overhead and memory usage.

**Completion criterion:** Ordinary programs can use core features without manual memory management, and cyclic garbage is reclaimable ([SPEC.md](SPEC.md) section 14).

**Blocked by:** nothing in section 21; U3, U7 and U8 are all resolved ([DECISIONS.md](DECISIONS.md) D026, D022, D025). The memory-management algorithm is also still unchosen ([RUNTIME.md](RUNTIME.md) section 8) and belongs before this phase, not inside it — and it must handle objects reachable from several bindings at once, since collections alias by default ([DECISIONS.md](DECISIONS.md) D013).

## Phase 7: Developer Experience

- Complete the initial `dovik` CLI.
- Support building, running, checking, and testing programs.
- Add consistent source formatting.
- Improve diagnostics and machine-readable output.
- Document installation and basic usage.

**Completion criterion:** Developers can create, compile, debug, and test small single-file programs reliably. Multi-file projects arrive in Phase 9.

**Blocked by:** nothing in section 21. The JSON diagnostic schema and the other CLI open decisions ([CLI.md](CLI.md) section 6) should be settled here.

## Phase 8: Performance and Stability

- Establish repeatable benchmarks.
- Profile compiler and runtime performance.
- Optimize based on measured bottlenecks.
- Test memory safety and runtime correctness.
- Establish release and compatibility practices.

**Completion criterion:** Performance claims are supported by reproducible results published with their environment and configuration ([BENCHMARKS.md](BENCHMARKS.md)). Until this phase completes, Dovik publishes no performance numbers at all.

**Blocked by:** nothing in section 21. Reclamation-pause measurement depends on the memory strategy chosen in Phase 6.

## Phase 9: Modules and Larger Projects

- Define module and import semantics. *(done — [SPEC.md](SPEC.md) 22, [DECISIONS.md](DECISIONS.md) D021)*
- Support larger multi-file programs. *(done — the entry file's import graph compiles as one program)*
- Introduce project configuration.
- Establish dependency management.
- Improve incremental compilation where practical.

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

Current position: **Phase 1 complete; Phase 2 not started.** No Dovik program has been compiled. The decisions that gated Phases 2 through 4 are all settled; the questions that remain gate Phases 5 and 6.

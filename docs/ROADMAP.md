# Dovik Development Roadmap

**Status:** Draft

## Phase 1: Define the Language

- Establish the language vision and core specification.
- Define variables, types, functions, control flow, and collections.
- Document compiler architecture, runtime requirements, CLI, and platform goals.
- Resolve essential language-design decisions before implementation.

**Completion criterion:** The initial language specification is clear enough to implement consistently.

## Phase 2: Compiler Foundation

- Set up the Rust project and development workflow.
- Implement a lexer and parser.
- Build an abstract syntax tree (AST).
- Report syntax errors with source locations.
- Add automated tests for valid and invalid source code.

**Completion criterion:** Dovik can parse source files and report useful syntax errors.

## Phase 3: Type System

- Implement name resolution and scope rules.
- Implement static type checking and type inference.
- Validate function calls, assignments, and return values.
- Produce clear diagnostics for type errors.

**Completion criterion:** Invalid programs are rejected before code generation.

## Phase 4: First Native Executables

- Introduce an intermediate representation (IR).
- Generate LLVM IR.
- Produce native executables for Linux x86-64.
- Support basic integer arithmetic, variables, and control flow.
- Automate compilation and execution tests.

**Completion criterion:** A Dovik program can compile and run as a native executable.

## Phase 5: Core Language

- Implement functions and recursion.
- Implement strings, collections, and essential operators.
- Establish consistent runtime behavior.
- Improve error messages and test coverage.

**Completion criterion:** Dovik supports its documented core language features.

## Phase 6: Runtime and Standard Library

- Implement memory management.
- Add console and file I/O.
- Implement essential collection and string operations.
- Establish a stable runtime interface.
- Measure runtime overhead and memory usage.

**Completion criterion:** Ordinary programs can use core features without manual memory management.

## Phase 7: Developer Experience

- Complete the initial `dovik` CLI.
- Support building, running, checking, and testing programs.
- Add consistent source formatting.
- Improve diagnostics and machine-readable output.
- Document installation and basic usage.

**Completion criterion:** Developers can create, compile, debug, and test small projects reliably.

## Phase 8: Performance and Stability

- Establish repeatable benchmarks.
- Profile compiler and runtime performance.
- Optimize based on measured bottlenecks.
- Test memory safety and runtime correctness.
- Establish release and compatibility practices.

**Completion criterion:** Performance claims are supported by reproducible results.

## Phase 9: Modules and Larger Projects

- Define module and import semantics.
- Introduce project configuration.
- Establish dependency management.
- Support larger multi-file programs.
- Improve incremental compilation where practical.

**Completion criterion:** Dovik can support maintainable multi-module applications.

## Phase 10: Platform Expansion

- Evaluate additional processor architectures and operating systems.
- Add Windows and macOS support.
- Investigate WebAssembly.
- Design mobile platform integrations.
- Validate portability of the runtime and standard library.

**Completion criterion:** Each new target has working builds, tests, and documented limitations.

## Phase 11: Self-Hosting

- Implement enough of Dovik to express substantial compiler functionality.
- Develop a Dovik implementation of the compiler.
- Bootstrap it using the existing Rust-based compiler.
- Verify that the resulting compiler can rebuild itself.
- Maintain reproducible bootstrap tests.

**Completion criterion:** Dovik can compile its compiler without relying on the Rust implementation for the final bootstrap stage.

## Development Principles

- Prioritize correctness over premature optimization.
- Implement one small, testable feature at a time.
- Keep the specification synchronized with actual behavior.
- Avoid implementing future platform features before the foundation is stable.
- Make architectural decisions explicit and revise them when evidence justifies it.
- Keep the compiler understandable and maintainable.

## Status

This roadmap is provisional. Phases may overlap or change as technical constraints become clearer. Completion is determined by working implementations and tests, not by documentation alone.

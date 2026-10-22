# Dovik Vision

**Status:** Language design and planning. Nothing described here is implemented.

This document states intent. [SPEC.md](SPEC.md) states the rules, and it is authoritative wherever the two could be read as disagreeing.

## 1. Purpose

Dovik is an experimental, statically typed programming language designed to combine simple, readable syntax with native compilation, efficient memory usage, and broad cross-platform capabilities.

## 2. Core Vision

Make software development simple for humans while allowing the compiler and runtime to handle complexity behind the scenes.

## 3. Core Principles

1. **Simplicity:** readable syntax with as few unnecessary rules as possible.
2. **Static typing:** catch type errors before program execution.
3. **Type inference:** infer types automatically whenever the compiler can determine them safely.
4. **Native performance:** compile to efficient machine code and measure performance against established languages.
5. **Memory efficiency:** minimize runtime overhead and manage memory automatically.
6. **Portability:** build toward a common language for CLI, backend, web, mobile, and desktop development.
7. **Developer experience:** provide clear errors, predictable tooling, and reliable testing.
8. **AI-agent compatibility:** make programs easy for coding agents to write, compile, debug, and verify.
9. **Correctness:** prioritize predictable behavior and reliable programs over premature optimization.
10. **Long-term independence:** develop Dovik's own language tooling, runtime, and standard library, with eventual compiler self-hosting.

## 4. Intended Users

- Developers who value simple, readable syntax.
- Systems and backend developers who need compiled performance.
- Developers building cross-platform software.
- Beginners who want a straightforward language.
- AI coding agents that need reliable compiler feedback.

## 5. Initial Scope

The first implementation will focus on small, single-file native CLI programs on Linux x86-64. Multi-file programs are not expressible, because Dovik has no module system yet ([SPEC.md](SPEC.md) section 16).

The initial version should establish:

- A lexer and parser.
- A statically typed semantic analyzer.
- Automatic type inference.
- Functions and control flow.
- Native executable generation.
- Essential runtime functionality.
- Automated tests and actionable diagnostics.

## 6. Long-Term Scope

Potential future targets include backend services, web applications, WebAssembly, Android, iOS, desktop applications, and additional operating systems.

These are long-term goals, not claims of existing support.

## 7. Non-Goals

- Guaranteeing that Dovik outperforms C in every workload.
- Supporting every platform in the first release.
- Adding features without a clear use case.
- Building a complex runtime before measuring the need.
- Prioritizing novelty over simplicity and correctness.

## 8. Success Criteria

Dovik should demonstrate its value through measurable improvements in developer experience, correctness, memory consumption, startup time, execution speed, and compilation reliability.

Performance claims must be supported by reproducible benchmarks.

## 9. Project Status

Language design and planning.

The compiler, runtime, and standard library are not yet established. No Dovik program has ever been compiled.

The decisions that gated the first compiler stages are settled. The questions that remain open gate the runtime and standard library rather than the frontend, and they are listed in [SPEC.md](SPEC.md) section 21.

## 10. Naming

Project name: Dovik

Source extension: `.dovk`

CLI command: `dovik`

The name is a working project name; trademark and availability checks have not been completed.

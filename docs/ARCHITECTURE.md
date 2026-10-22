# Dovik Compiler Architecture

- **Status:** Initial design — nothing here is implemented
- **Implementation language:** Rust
- **Code generation backend:** LLVM

Language semantics are defined by [SPEC.md](SPEC.md), which is authoritative. This document covers only how the compiler is built. Where an implementation choice would change observable program behavior, the specification decides, not this document.

## 1. Overview

Dovik will use a compiler architecture that separates language semantics from machine-code generation.

The initial compiler will target native Linux x86-64 executables ([DECISIONS.md](DECISIONS.md) D015).

## 2. Compilation Pipeline

1. **Source input:** Read `.dovk` files.
2. **Lexer:** Convert source characters into tokens.
3. **Parser:** Build an abstract syntax tree (AST).
4. **Signature collection:** Record every module-level function's parameter types and declared return type before any body is examined. This is what makes mutual recursion possible ([SPEC.md](SPEC.md) 5.5) and must precede stage 5.
5. **Name resolution:** Bind every identifier to a declaration and enforce the scope rules in [SPEC.md](SPEC.md) section 9 — the declaration-versus-reassignment rule (9.2), the shadowing rule (9.3), and the local-binding rules (9.6). Function names resolve module-wide; variables resolve in source order ([SPEC.md](SPEC.md) 8.5).
6. **Type checking and inference:** Infer variable and return types, identify recursive cycles in the call graph, require an explicit return annotation for any function whose return type the cycle makes uninferable, and reject every compile-time error listed in [SPEC.md](SPEC.md) 15.1.
7. **Intermediate representation:** Lower validated programs into a Dovik-controlled IR.
8. **Optimization:** Apply appropriate compiler optimizations.
9. **LLVM backend:** Translate the IR into LLVM IR and generate native machine code.
10. **Executable generation:** Produce a runnable native program.

## 3. Architectural Principles

- Language semantics must not depend on LLVM-specific behavior. "Whatever LLVM does" is never an answer to a semantic question.
- Each compiler stage must be testable independently.
- Type errors must be detected before code generation.
- Optimization must preserve program behavior. The checked-arithmetic errors required by [SPEC.md](SPEC.md) section 7 must hold at every optimization level: integer overflow (7.2), division by zero for both `Int` and `Float` operands (6.2), and remainder by zero (6.3). A constant-folded expression that overflows or divides by zero must be rejected at compile time rather than folded to a wrapped value, an infinity, or a NaN.
- `/` does not lower to a bare LLVM `fdiv`. A zero-divisor check is part of the operator's semantics, not an optional guard, and LLVM's IEEE 754 default behavior is not Dovik's.
- Bounds checks required by [SPEC.md](SPEC.md) 10.4 and 11.3 must survive optimization. Eliding one is permitted only where the index is provably in range.
- Diagnostics must identify source locations and explain errors.
- Compiler architecture should support additional target platforms over time, without any document claiming support before builds and tests exist.

## 4. Intermediate Representation

Dovik will maintain its own intermediate representation between semantic analysis and LLVM code generation.

This creates a boundary between the language frontend and the backend.

The initial IR design will be determined when implementing the compiler pipeline. It must be able to represent the checked arithmetic operations explicitly, so that a later optimization pass cannot silently discard a required check.

## 5. Runtime Integration

The compiler and runtime must cooperate to provide strings, collections, I/O, and automatic memory management.

LLVM does not provide Dovik's memory-management semantics automatically. Those must be designed and implemented separately.

## 6. Initial Target

The proposed first target is Linux x86-64.

Other operating systems, CPU architectures, WebAssembly, and mobile platforms require additional implementation and validation.

## 7. Open Decisions

These are implementation decisions and do not affect language semantics:

- LLVM version and choice of Rust bindings.
- LLVM IR lowering strategy, including how overflow, division-by-zero, and bounds checks are emitted.
- Indentation handling in the lexer: how indent and dedent are tokenized, and how the four-space rule ([SPEC.md](SPEC.md) 2.4) is diagnosed.
- Debug information.
- Linking strategy, and whether the runtime is linked statically or dynamically.
- Runtime ABI and memory-management integration.
- Cross-compilation strategy.
- Optimization defaults.

Separately, several **language** questions remain open and block later compiler work. They are tracked in [SPEC.md](SPEC.md) section 21, not here. None of them blocks stages 1 through 6: U1, U2, and U6 have been resolved (see [DECISIONS.md](DECISIONS.md) D013, D020, D019), so the frontend through type checking is unblocked. U3 constrains the string representation and U7 the failure modes, both of which land in the runtime.

## 8. Status

This architecture is a starting point, and no part of it is implemented. Detailed implementation decisions will be finalized before the corresponding compiler components are built.

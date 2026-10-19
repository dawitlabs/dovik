# Dovik Compiler Architecture

**Status:** Initial design  
**Implementation language:** Rust  
**Code generation backend:** LLVM

## 1. Overview

Dovik will use a compiler architecture that separates language semantics from machine-code generation.

The initial compiler will target native Linux x86-64 executables.

## 2. Compilation Pipeline

1. **Source input:** Read `.dovk` files.
2. **Lexer:** Convert source characters into tokens.
3. **Parser:** Build an abstract syntax tree (AST).
4. **Semantic analysis:** Resolve names, infer types, and check type correctness.
5. **Intermediate representation:** Lower validated programs into a Dovik-controlled IR.
6. **Optimization:** Apply appropriate compiler optimizations.
7. **LLVM backend:** Translate the IR into LLVM IR and generate native machine code.
8. **Executable generation:** Produce a runnable native program.

## 3. Architectural Principles

- Language semantics must not depend on LLVM-specific behavior.
- Each compiler stage must be testable independently.
- Type errors must be detected before code generation.
- Optimization must preserve program behavior.
- Diagnostics must identify source locations and explain errors.
- Compiler architecture should support additional target platforms over time.

## 4. Intermediate Representation

Dovik will maintain its own intermediate representation between semantic analysis and LLVM code generation.

This creates a boundary between the language frontend and the backend.

The initial IR design will be determined when implementing the compiler pipeline.

## 5. Runtime Integration

The compiler and runtime must cooperate to provide strings, collections, I/O, and automatic memory management.

LLVM does not provide Dovik's memory-management semantics automatically. Those must be designed and implemented separately.

## 6. Initial Target

The proposed first target is Linux x86-64.

Other operating systems, CPU architectures, WebAssembly, and mobile platforms require additional implementation and validation.

## 7. Open Decisions

- LLVM version and Rust bindings.
- LLVM IR lowering strategy.
- Debug information.
- Linking strategy.
- Runtime ABI and memory-management integration.
- Cross-compilation strategy.
- Optimization defaults.

## 8. Status

This architecture is a starting point. Detailed implementation decisions will be finalized before the corresponding compiler components are built.

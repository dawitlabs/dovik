# Dovik Design Decisions

**Status:** Draft

This document records important design choices, their reasoning, and decisions that remain open.

## D001: Bootstrap Language

**Decision:** Use Rust to implement the initial compiler.

**Reason:** Rust provides strong type safety, good performance, and mature tooling.

**Status:** Accepted provisionally.

## D002: Code Generation Backend

**Decision:** Use LLVM for initial native code generation.

**Reason:** LLVM provides optimization infrastructure and support for multiple target architectures.

**Trade-off:** LLVM adds a dependency and complexity. Dovik's language semantics must remain independent of LLVM.

**Status:** Accepted provisionally.

## D003: Static Typing

**Decision:** Dovik is statically typed with type inference.

**Reason:** Catch type errors before execution without requiring annotations everywhere.

**Status:** Accepted.

## D004: Variable Declarations

**Decision:** Use assignment syntax such as `age = 23`, with optional type annotations such as `age: Int = 23`.

**Reason:** Keep common code concise while allowing explicit types when useful.

**Status:** Proposed.

## D005: Indentation-Based Syntax

**Decision:** Use indentation to define blocks, without mandatory braces or semicolons.

**Reason:** Keep source code readable and reduce syntax overhead.

**Status:** Proposed.

## D006: Automatic Memory Management

**Decision:** Ordinary programs should not require manual memory deallocation.

**Reason:** Reduce memory-management mistakes and simplify development.

**Trade-off:** The implementation must balance memory

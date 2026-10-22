# Dovik Runtime

- **Status:** Draft — nothing here is implemented
- **Goal:** Automatic memory management with low overhead.

Language semantics are defined by [SPEC.md](SPEC.md), which is authoritative. This document covers runtime goals and the decisions still open. The runtime may not introduce observable behavior the specification does not define.

## 1. Purpose

The Dovik runtime provides the functionality required by compiled programs that cannot be handled by generated machine code alone.

It should remain small, predictable, and efficient.

## 2. Responsibilities

The runtime may provide:

- String operations.
- Dynamic-size collections.
- Console input and output.
- File operations.
- Runtime error reporting.
- Memory allocation and reclamation.
- Support for standard-library functionality.

Simple programs should require as little runtime overhead as practical.

The runtime must raise exactly the runtime errors listed in [SPEC.md](SPEC.md) 15.2 — integer overflow, division by zero, remainder by zero, invalid numeric conversion, out-of-bounds indexing, and missing dictionary keys — and must terminate the program rather than continue with a corrupted value. There is no mechanism for a program to catch one.

Division by zero is an error for `Float` operands as well as `Int`, including `0 / 0` ([SPEC.md](SPEC.md) 6.2). The runtime must not substitute an IEEE 754 infinity or NaN. NaN remains reachable from other float operations, so the runtime must still represent and print it ([SPEC.md](SPEC.md) 7.3).

## 3. Memory Management Goals

- Developers should not normally free objects manually.
- Memory management must preserve program correctness.
- Common operations should have low overhead.
- Unreachable objects should eventually be reclaimed.
- Cyclic references must not cause permanent memory leaks.
- Memory usage must be measured under realistic workloads.

## 4. Primitive Values

`Int`, `Float`, and `Bool` are value types ([SPEC.md](SPEC.md) 3.4). They should use efficient representations and avoid unnecessary heap allocation.

The compiler should optimize values where doing so preserves language semantics.

## 5. Heap Objects

`String`, `List`, and `Dict` are reference types ([SPEC.md](SPEC.md) 3.4), so assignment and argument passing share one object rather than copying. The runtime must preserve that aliasing exactly: a mutation through any reference is visible through every reference to that object, while rebinding a variable affects no other reference ([SPEC.md](SPEC.md) 9.6). A copy-on-write representation is permitted only if it is unobservable.

Because aliasing is observable, an object can be reachable from several bindings at once, which the memory-management strategy must account for — shared ownership is the normal case, not the exception.

Their concrete representation, allocation strategy, and reclamation strategy remain undecided.

The string representation is constrained by an unresolved language question: [SPEC.md](SPEC.md) 10.4 indexes strings by Unicode scalar value over UTF-8 storage, which is O(n) per index. See U3 in [SPEC.md](SPEC.md) section 21 — that question must be settled before the string representation is chosen, not after.

## 6. Runtime Errors

Runtime failures must identify what failed and where, and must terminate predictably.

Two failure modes have no specified behavior at all: call-stack exhaustion from deep recursion, and out-of-memory. Both are tracked as U7 in [SPEC.md](SPEC.md) section 21. Until they are decided, a deeply recursive Dovik program's failure mode is undefined, which undercuts any claim that Dovik's runtime errors are predictable.

## 7. Performance Measurements

Runtime evaluation should include:

- Peak resident memory.
- Allocation overhead.
- Startup time.
- Execution speed.
- Runtime and executable size.

Measurements should use reproducible programs and documented environments.

## 8. Open Decisions

- Garbage collection versus other automatic memory-management strategies. No algorithm has been chosen, so no document should assume one — including benchmark plans ([BENCHMARKS.md](BENCHMARKS.md)).
- Cyclic reference handling. [SPEC.md](SPEC.md) section 14 requires that cycles be reclaimable, which rules out plain non-atomic reference counting on its own.
- Object layout.
- String representation — blocked on U3.
- Collection representation.
- Stack exhaustion and out-of-memory behavior — blocked on U7.
- Runtime linking and distribution.
- Thread safety and concurrency. The runtime promises no thread safety for mutable collections, and Dovik has no concurrency model ([SPEC.md](SPEC.md) section 16).

## 9. Status

This document defines goals, not a finalized memory-management implementation. No runtime code exists.

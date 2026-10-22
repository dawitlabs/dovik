# Dovik Runtime

- **Status:** Draft — nothing here is implemented
- **Goal:** Automatic memory management with low overhead.
- **Implementation:** C, compiled to bitcode and linked into each program with full LTO

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

Their concrete representation and allocation strategy are now implemented:
a heap object carries a reference count, and the compiler emits retain and release
around every binding that holds one. See section 11.

The string representation is constrained by an unresolved language question: [SPEC.md](SPEC.md) 10.4 indexes strings by Unicode scalar value over UTF-8 storage, which is O(n) per index. See U3 in [SPEC.md](SPEC.md) section 21 — that question must be settled before the string representation is chosen, not after.

## 6. Runtime Errors

Runtime failures must identify what failed and where, and must terminate predictably.

Both of the failure modes that once had no specified behaviour now have one ([SPEC.md](SPEC.md) 15.2, [DECISIONS.md](DECISIONS.md) D022). Call-stack exhaustion reports `DVK5009` and out-of-memory reports `DVK5010`; both terminate with a diagnostic and exit 1 rather than on a signal, and neither reports a source location.

Stack exhaustion is caught rather than counted. `dovik_start`, called first from `main`, installs a `SIGSEGV` and `SIGBUS` handler on an alternate signal stack — an alternate stack is required, because the faulting thread has no stack left to run a handler on — and records the low end of the main thread's stack from `pthread_getattr_np`. A fault within 256 KiB below that address is a stack overflow. The handler writes its diagnostic with `write(2)` rather than `fprintf`, because only the former is safe to call from a signal handler, and exits with `_exit(2)`.

A fault anywhere else reports `DVK9001`, which says a compiler defect rather than a program error. That is the honest reading: a Dovik program cannot form a bad pointer, so any other fault is the runtime's fault. `DVK9001` is an implementation diagnostic and deliberately not in [SPEC.md](SPEC.md) 15.2, which enumerates errors a *program* can cause.

Out-of-memory is reported from `checked_alloc`, the single allocation point, which is why one check covers every allocation the runtime makes.

## 7. Performance Measurements

Runtime evaluation should include:

- Peak resident memory.
- Allocation overhead.
- Startup time.
- Execution speed.
- Runtime and executable size.

Measurements should use reproducible programs and documented environments.

## 8. Open Decisions

- Whether reference counting remains the strategy, or is replaced once the language gains features that can build a cycle. See section 11.
- Cyclic reference handling, once cycles become constructible. They are not today; section 11 explains why.
- Object layout.
- String representation — blocked on U3.
- Collection representation.
- The recursion depth a native build reaches, which follows `RLIMIT_STACK` rather than anything Dovik chooses.
- Runtime linking and distribution.
- Thread safety and concurrency. The runtime promises no thread safety for mutable collections, and Dovik has no concurrency model ([SPEC.md](SPEC.md) section 16).

## 9. Status

This document defines goals, not a finalized memory-management implementation. No runtime code exists.

---

## 11. Implemented Strategy: Reference Counting

Every heap object — `String`, `List`, `Dict` — carries a non-atomic reference count.
The compiler emits a retain where a binding takes ownership and a release where one
gives it up: a slot store releases what it displaces, a function releases its frame on
every exit, a container retains what is put into it and releases what it drops, and a
returned value is retained for the caller.

Counts are non-atomic because Dovik has no concurrency. Adding threads invalidates
that and nothing else here.

### Why this satisfies section 14 today

[SPEC.md](SPEC.md) section 14 requires that unreachable values, *including those in
reference cycles*, be eventually reclaimable. Plain reference counting cannot reclaim
a cycle, so on its face it does not qualify.

It qualifies because the initial language cannot build one. Types are `Int`, `Float`,
`Bool`, `String`, `List<T>` and `Dict<K, V>`, with no user-defined and no recursive
types. A cycle needs `T` to transitively contain `List<T>`, which needs an infinitely
nested type annotation, and no program can write one.

**This is a precondition, not a property of reference counting.** The first feature
that lets a value reach itself — a named type, a recursive type, a closure capturing
its own binding, or a reference cell — breaks it, and reclamation has to be revisited
in the same change. Nothing in the compiler detects this; it is a standing obligation
on whoever adds such a feature.

### Cost

Measured in [BENCHMARKS.md](BENCHMARKS.md) section 11. Reclaiming memory took the
string workload from 197 MB to 3 MB and, because it stopped touching 200 MB, from
173 ms to 19 ms. The allocation-churn workload got ~30% slower: it creates and drops
200000 short-lived lists, pays retain and release on each, and was never memory-bound.
That is the honest shape of the trade.

### Verification

`tests/native.rs` compiles programs with `DOVIK_CHECK_LEAKS=1` and asserts the runtime
reports zero live objects at exit. The suite covers concatenation loops, list churn,
string parameters, returned strings, nested collections and dictionary overwrites.
A leak and an over-release both fail it — the second by crashing — which is what makes
the retain and release placement checkable rather than merely argued.

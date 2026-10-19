# Dovik Runtime

**Status:** Draft  
**Goal:** Automatic memory management with low overhead.

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

## 3. Memory Management Goals

- Developers should not normally free objects manually.
- Memory management must preserve program correctness.
- Common operations should have low overhead.
- Unreachable objects should eventually be reclaimed.
- Cyclic references must not cause permanent memory leaks.
- Memory usage must be measured under realistic workloads.

## 4. Primitive Values

Booleans and numeric values should use efficient representations and avoid unnecessary heap allocations.

The compiler should optimize values where doing so preserves language semantics.

## 5. Heap Objects

Strings and collections may require heap allocation.

Their representation, ownership rules, mutation behavior, and reclamation strategy remain undecided.

## 6. Runtime Errors

Runtime failures must produce useful diagnostics where practical.

The exact behavior for out-of-memory conditions and other unrecoverable failures remains to be specified.

## 7. Performance Measurements

Runtime evaluation should include:

- Peak resident memory.
- Allocation overhead.
- Startup time.
- Execution speed.
- Runtime and executable size.

Measurements should use reproducible programs and documented environments.

## 8. Open Decisions

- Garbage collection versus other automatic memory-management strategies.
- Cyclic reference handling.
- Object layout.
- String representation.
- Collection representation.
- Runtime linking and distribution.
- Thread safety and concurrency.

## 9. Status

This document defines goals, not a finalized memory-management implementation.

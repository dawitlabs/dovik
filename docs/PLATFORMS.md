# Dovik Platform Support

**Status:** Draft — no platform is supported yet

Linux x86-64 is the initial *target*, not a working one: no compiler exists, so no platform is currently supported. Every other platform in this document is a goal.

## 1. Goal

Dovik aims to support multiple platforms through one language and a consistent developer experience.

Platform support will expand gradually as the compiler and runtime mature.

## 2. Initial Target

The first target is:

- Linux x86-64
- Native executable output
- LLVM-based code generation
- A minimal runtime

The initial compiler will prioritize correctness and reliability on this target before expanding ([DECISIONS.md](DECISIONS.md) D015).

## 3. Planned Platforms

Long-term targets include:

- **Linux:** Native command-line tools and backend services.
- **Windows:** Native applications and command-line tools.
- **macOS:** Native applications and command-line tools.
- **Web:** WebAssembly-based applications where practical.
- **Mobile:** Android and iOS applications through suitable platform integrations.
- **Desktop:** Cross-platform desktop applications through future libraries and tooling.

These are goals, not current capabilities.

## 4. Platform Independence

The language specification should remain independent of operating systems and processor architectures wherever possible.

Platform-specific functionality belongs in the compiler backend, runtime, standard library, or platform integrations.

Programs that use platform-specific APIs may require platform-specific code.

## 5. Standard Library

The standard library should provide portable interfaces for common operations, including:

- Console input and output
- File operations
- Strings and collections
- Time and dates
- Networking
- Concurrency, when supported

Availability may differ by platform.

## 6. Cross-Compilation

Cross-compilation is a future goal. The compiler should eventually be able to produce executables for supported targets other than the host platform.

This requires compatible target code generation, runtime components, and linking tools.

## 7. Compatibility

Dovik should define which language features are portable and document platform limitations clearly.

Unsupported features should produce clear diagnostics rather than failing unpredictably.

## 8. Open Decisions

- Which platform comes after Linux x86-64?
- Which processor architectures should be supported?
- How should mobile applications integrate with native platform APIs?
- Should WebAssembly be a primary target or a separate compilation target?
- How should platform-specific dependencies be managed?

## 9. Status

This document describes planned platform support. None of it is implemented — not Linux, and certainly not Windows, macOS, WebAssembly, Android, iOS, desktop, or cross-compilation. A target counts as supported only once it has working builds and a passing test suite.

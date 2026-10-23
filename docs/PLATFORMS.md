# Dovik Platform Support

**Status:** Linux x86-64 is supported. WebAssembly runs, with one documented limitation.

Both meet the bar section 9 sets: working builds and a passing test suite. Every other platform in this document is a goal, and no other target has been attempted.

## 1. Goal

Dovik aims to support multiple platforms through one language and a consistent developer experience.

Platform support will expand gradually as the compiler and runtime mature.

## 2. Initial Target

The first target is:

- Linux x86-64
- Native executable output
- LLVM-based code generation
- A minimal runtime

All four exist. The compiler emits LLVM IR as text and links it with `clang` against a C runtime, and the test suite builds and runs native executables on this target.

Correctness and reliability on this target come before expanding ([DECISIONS.md](DECISIONS.md) D015).

## 3. Planned Platforms

Long-term targets include:

- **Linux:** Native command-line tools and backend services. *(command-line tools work; a backend service needs networking, which the standard library does not have)*
- **Windows:** Native applications and command-line tools.
- **macOS:** Native applications and command-line tools.
- **Web:** WebAssembly-based applications where practical. *(`--target wasm` builds a WASI module today; a browser needs a JS host and is not built — [PROPOSAL-WASM.md](PROPOSAL-WASM.md) 5)*
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

Availability may differ by platform. Of this list, console I/O, files, strings, collections,
and time exist today ([STDLIB.md](STDLIB.md)). **Networking and concurrency do not exist at
all** — there is no socket, no HTTP client or server, and no thread. Nothing in Dovik can
reach the network, so a web server, a web client, and a browser target are all out of reach
until that changes. Neither has a design yet.

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
- ~~Should WebAssembly be a primary target or a separate compilation target?~~ A separate one,
  via WASI, and built: [PROPOSAL-WASM.md](PROPOSAL-WASM.md).
- How should platform-specific dependencies be managed?

## 9. Status

Linux x86-64 and `wasm32-wasip1` are implemented and tested. Windows, macOS, Android, iOS, desktop, and general cross-compilation are not, and none of them has been started. A target counts as supported only once it has working builds and a passing test suite.

**The one documented limitation.** On WebAssembly, stack exhaustion is detected by counting
call frames rather than by measuring the stack, because no host exposes it. The default limit
of 2,000 frames sits about a factor of two below the fattest frame measured, so a function with
far more locals can still exhaust the host stack before the counter fires, which traps the
module with no `DVK5009`. [PROPOSAL-WASM.md](PROPOSAL-WASM.md) section 7 has the measurements.
Native is unaffected; its signal handler measures the real stack.

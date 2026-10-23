# Proposal: WebAssembly as a Target

**Status:** **Draft, not ratified and not implemented.** No WebAssembly build exists. This is
the design work [ROADMAP.md](ROADMAP.md) Phase 10 and [PLATFORMS.md](PLATFORMS.md) section 8
record as not started. Nothing here is authoritative; [SPEC.md](SPEC.md) is.

Everything measured below was measured on this machine with clang 22.1.8, and each number
says how.

## 1. What already works, and what does not

`clang --print-targets` lists `wasm32` and `wasm64`, so the LLVM side needs nothing: the
backend emits LLVM IR as text and the target triple is an argument. That is the whole of the
good news.

The runtime is the problem. Compiling it for a bare `wasm32-unknown-unknown` stops on the
first line:

```
$ clang --target=wasm32-unknown-unknown -c compiler/runtime-c/dovik_rt.c
compiler/runtime-c/dovik_rt.c:3:10: fatal error: 'dirent.h' file not found
```

That is not one missing header. Compiling a file that includes nothing but one header each
time, for the same target, finds only `stdint.h`:

| Header | Present on bare `wasm32` |
| --- | --- |
| `stdint.h` | yes — compiler-provided |
| `stdio.h`, `stdlib.h`, `string.h`, `time.h` | **no** |
| `unistd.h`, `dirent.h`, `sys/stat.h` | **no** |
| `pthread.h`, `signal.h` | **no** |

A bare target has no libc at all, so `malloc`, `memcpy`, `snprintf`, `fopen` and `getenv` are
all absent. The runtime uses every one of them.

## 2. The toolchain decision

Two routes, and they are not close in cost.

**`wasm32-wasip1`, against wasi-libc.** WASI supplies a libc, so `stdio.h`, `stdlib.h`,
`string.h` and `time.h` arrive unchanged, file and directory operations map onto preopened
directories, and `argument_count`/`argument` map onto `args_get`. Most of the runtime compiles
as it stands. It needs a sysroot that is not installed here — the `wasi-sdk` or `wasi-libc`
package — and a runtime to execute the result: `wasmtime`, `wasmer`, or Node's WASI support.
**Neither is installed**, so this route starts with a dependency decision, not with code.

**`wasm32-unknown-unknown`, freestanding.** No libc, so the port writes its own allocator,
its own `memcpy` and `memcmp`, and reaches the host through imported functions for output.
That is a second runtime to maintain beside the C one, and it would diverge. It buys a module
that runs in a browser with no WASI shim.

**Recommendation: WASI first.** It is the smaller change by a wide margin, and it keeps one
runtime. A browser target can come later as a second WASI-shimmed or freestanding variant,
once there is a reason to want one. This also answers
[PLATFORMS.md](PLATFORMS.md) section 8's open question — WebAssembly is a *separate
compilation target*, not a primary one.

## 3. What breaks that is not a header

Three things survive neither route unchanged, and the first is a specification problem rather
than a porting problem.

### 3.1 Stack exhaustion cannot be detected the way it is now

[SPEC.md](SPEC.md) 15.2 requires that call-stack exhaustion "report what failed" and
"terminate predictably — never continue with a corrupted value, and never fault." The runtime
delivers that with a `SIGSEGV` handler on an alternate signal stack, comparing the faulting
address against the stack bounds it reads through `pthread_getattr_np`
([DECISIONS.md](DECISIONS.md) D022).

WebAssembly has no signals, no `sigaltstack`, and no faulting address to inspect. Exhausting
the wasm call stack is an unrecoverable trap: the module aborts, and the host sees a trap with
no Dovik diagnostic attached. `DVK5009` would simply not be reported.

So a WASI build either violates 15.2 or the compiler emits an explicit depth check. The second
is a code-generation change with a measurable cost on every call, on every target or on one —
and "observable program behavior does not depend on which algorithm the runtime uses"
([SPEC.md](SPEC.md) 14) has a sibling expectation here that a diagnostic does not depend on
the target either.

**This is the decision that gates the rest.** It is not settled by picking a toolchain.

### 3.2 Reference counting is fine; the fault handler is not

Non-atomic counts stay correct, because WASI preview 1 has no threads
([RUNTIME.md](RUNTIME.md) section 11 depends on exactly that). `DVK9001`, the
"runtime faulted, this is a compiler defect" path, disappears with the signal handler, which
is a loss of diagnosis rather than of correctness.

### 3.3 Part of the standard library has no meaning in a sandbox

Directory operations and file I/O work under WASI only for directories the host preopens. A
module given no preopened directory gets `DVK5007` for every path, which is honest but makes
`list_dir` and `create_dir` unusable rather than absent. [PLATFORMS.md](PLATFORMS.md) section
5 already says availability may differ by platform; this is the first case where it actually
would, and it needs documenting per function in [STDLIB.md](STDLIB.md) rather than as a
footnote.

## 4. What this costs to build

Ordered, with the gate first:

1. **Decide 3.1.** Either an explicit depth counter in the backend, or an amendment to 15.2
   admitting a target where stack exhaustion traps without a diagnostic. No code before this.
2. **Add the dependency.** `wasi-sdk` for the sysroot and one of `wasmtime`/`wasmer` to
   execute. Both are new build-time dependencies for the project and neither is needed to
   build for Linux.
3. **Make the runtime's POSIX use conditional.** `install_fault_handler` and the `pthread`
   stack lookup go behind a target check; everything else is expected to compile.
4. **Thread the target through the CLI and the linker invocation.** `link` in
   `cli/pipeline.rs` hardcodes a native `clang` call; it needs a target triple and a
   per-target runtime archive, which `build.rs` currently builds exactly one of.
5. **Test it.** [PLATFORMS.md](PLATFORMS.md) section 9 is the bar: working builds and a
   passing test suite, or the target is not supported. That means running the existing native
   corpus under a wasm runtime, not merely linking a module.

Steps 3 through 5 are the work. Step 1 is the decision, and step 2 is a question for whoever
owns what this repository depends on.

## 5. What this deliberately leaves out

- **A browser target.** Needs a JS host, an output path that is not `fwrite`, and a story for
  files. Separate proposal, if ever.
- **`wasm64`.** Listed by clang, with no reason to want it.
- **Threads.** WASI threads exist in later previews and would invalidate
  [RUNTIME.md](RUNTIME.md) section 11's non-atomic counts. Out of scope here and coupled to
  the concurrency model Dovik does not have ([SPEC.md](SPEC.md) section 16).

## 6. Open decisions

- Stack exhaustion under a target with no signals (3.1). **Gates everything.**
- Whether the project takes on `wasi-sdk` and a wasm runtime as dependencies.
- Whether per-target standard library availability is documented per function or per platform.
- Whether a depth check, if chosen, is emitted on every target for one diagnostic or only
  where signals are unavailable.

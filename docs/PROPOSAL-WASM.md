# Proposal: WebAssembly as a Target

**Status:** **Built.** `dovik build file.dovk --target wasm` produces a WebAssembly module
that runs under a WASI host, and `tests/wasm.rs` runs a corpus and every runtime check through
it against the interpreter. Section 7 records what the port taught that this design did not
predict, including one limitation that remains. Nothing here is authoritative;
[SPEC.md](SPEC.md) is.

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

Three things survive neither route unchanged. The first looked like a specification problem,
and turned out to be a missing mechanism; the other two are still open.

### 3.1 Stack exhaustion cannot be detected the way it is now — **resolved, D030**

[SPEC.md](SPEC.md) 15.2 requires that call-stack exhaustion "report what failed" and
"terminate predictably — never continue with a corrupted value, and never fault." The runtime
delivers that with a `SIGSEGV` handler on an alternate signal stack, comparing the faulting
address against the stack bounds it reads through `pthread_getattr_np`
([DECISIONS.md](DECISIONS.md) D022).

WebAssembly has no signals, no `sigaltstack`, and no faulting address to inspect. Exhausting
the wasm call stack is an unrecoverable trap: the module aborts, and the host sees a trap with
no Dovik diagnostic attached. `DVK5009` would simply not be reported.

This was recorded here as the decision gating everything else. It was already decided, by the
paragraph after 15.2's list: the depth "may differ between a native build and any other
execution of the same program", and "what is specified is the failure: a diagnostic and a
non-zero exit, not a signal." The specification never promised the mechanism.

So no amendment, and **the mechanism now exists** ([DECISIONS.md](DECISIONS.md) D030):
`--depth-guard` emits an explicit counter, limited by `DOVIK_MAX_DEPTH`, in exactly the
functions that can reach themselves — which the backend already knows, because it computes
call reachability for purity. `fib(30)` costs about 5%; a default build emits no guard at all.

It is tested on Linux today, which is the point: when a sysroot arrives, this part is done
rather than discovered.

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

1. ~~**Decide 3.1.**~~ **Done** — [DECISIONS.md](DECISIONS.md) D030, and the counter is built
   and tested.
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

All five are done. The dependency turned out smaller than step 2 feared: `wasi-libc` and
`wasi-compiler-rt` are in Arch's own repositories, and Node 26 ships a WASI host, so no
`wasi-sdk` and no `wasmtime` were needed. `build.rs` compiles a second runtime object when the
sysroot is present and emits an empty path when it is not, so a machine without wasi-libc
still builds the compiler and the wasm tests report that they were skipped.

## 7. What the port taught

**Section 1 was right about the headers and wrong about the scale.** Against
`--sysroot=/usr/share/wasi-sysroot`, exactly one thing failed to compile: the signal handler.
`dirent.h`, `pthread.h`, `stdio.h` and the rest all resolved. Fifteen errors, all of them in
`on_fault` and `install_fault_handler`, which `--depth-guard` had already made replaceable.

**The entry point is not called `main`.** This cost an hour and appears in no part of the
design above. wasi-libc's `crt1` calls `__main_argc_argv`, and clang performs that rename in
the C frontend — so a hand-written LLVM module that defines `main` links successfully and then
traps at `undefined_weak:main`, with no diagnostic that names the cause. `Target::entry_symbol`
is the fix, and the lesson is that a target is more than a triple and a sysroot.

**A frame counter cannot model a wasm stack.** The default limit of 100,000 from
[DECISIONS.md](DECISIONS.md) D030 was measured against native, and under Node a module traps
long before it. Measured, one recursive function per row:

| Locals in the frame | Frames before the host trapped |
| --- | --- |
| 2 | ~13,000 |
| 9 | ~6,000 |
| 25 | ~4,500 |

The product is not constant — 26,000, 54,000, 112,500 — so weighting the counter by frame size
would overcorrect as badly as not weighting it. The per-frame cost is dominated by host
overhead at small counts, and LLVM keeps most locals out of the shadow stack at `-O2`, so
`__builtin_frame_address` would not measure the real thing either.

**So the wasm default is 2,000 frames, and this is a documented limitation rather than a
solved problem.** It leaves roughly a factor of two below the fattest frame measured, and a
function with far more locals than that could still exhaust the host stack before the counter
fires — which traps the module with no `DVK5009`, and is the one place the wasm target does
not deliver what [SPEC.md](SPEC.md) 15.2 asks. `DOVIK_MAX_DEPTH` is the workaround, and
removing the limitation needs a host that reports stack headroom. Native is unaffected: the
signal handler measures the real stack, and its limit is unchanged.

## 5. What this deliberately leaves out

- **A browser target.** Needs a JS host, an output path that is not `fwrite`, and a story for
  files. Separate proposal, if ever.
- **`wasm64`.** Listed by clang, with no reason to want it.
- **Threads.** WASI threads exist in later previews and would invalidate
  [RUNTIME.md](RUNTIME.md) section 11's non-atomic counts. Out of scope here and coupled to
  the concurrency model Dovik does not have ([SPEC.md](SPEC.md) section 16).

## 6. Open decisions

- ~~Stack exhaustion under a target with no signals (3.1).~~ Settled as
  [DECISIONS.md](DECISIONS.md) D030 and implemented.
- ~~Whether the project takes on `wasi-sdk` and a wasm runtime as dependencies.~~ Neither was
  needed: `wasi-libc` and `wasi-compiler-rt` from the distribution, and a WASI host that Node
  already ships.
- Whether per-target standard library availability is documented per function or per platform.
  Still open; the test harness preopens `/`, so nothing has forced the question yet.
- How to make the depth guard track real stack use on a host that does not expose it
  (section 7). The 2,000-frame default is a margin, not an answer.
- Whether `wasm32-wasip2` and the component model are worth following.

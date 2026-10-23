# Proposal: The Targets After WebAssembly

**Status:** **Draft, not ratified and not implemented.** Linux x86-64 and `wasm32-wasip1` are
the supported targets ([PLATFORMS.md](PLATFORMS.md)). This file measures what the next ones
need and recommends an order, which is [ROADMAP.md](ROADMAP.md) Phase 10's remaining work and
the cross-compilation question [ARCHITECTURE.md](ARCHITECTURE.md) section 7 leaves open.

## 1. What a cross-compile attempt actually says

Every candidate target was given the same command — compile the runtime, nothing else — on this
machine with clang 22.1.8:

| Target | Result |
| --- | --- |
| `aarch64-unknown-linux-gnu` | falls into the **host's** glibc, dies on `gnu/stubs-32.h` |
| `x86_64-pc-windows-gnu` | `dirent.h` not found |
| `x86_64-pc-windows-msvc` | `dirent.h` not found |
| `aarch64-apple-darwin` | falls into the host's glibc, dies on `gnu/stubs-32.h` |
| `x86_64-apple-darwin` | reads **Linux's** `/usr/include/dirent.h` and fails to parse it |

**The lesson is the one WebAssembly already taught: a target is a sysroot, and clang silently
uses the wrong one rather than refusing.** The two Darwin rows are the clearest case — clang
accepted the triple, found the host's Linux headers, and produced a parse error deep inside
them. A triple alone compiles nothing.

`Target` in `compiler/src/target.rs` already carries a triple, an entry symbol, whether the
depth guard is needed, and an output extension, so adding a target is a variant plus a sysroot
plus a runtime object. The shape is right; the sysroots are the work.

## 2. The order, and why

**First: `aarch64-unknown-linux-gnu`.** The cheapest by a wide margin. Same operating system,
so the runtime's POSIX use — `dirent.h`, `sys/stat.h`, `unistd.h` — is unchanged, and a cross
sysroot is a packaged toolchain rather than a licensing question. This is the target that proves
`Target` generalizes beyond "Linux or wasm" without also needing a platform shim. It is also
the only one that could plausibly be tested in CI on a runner that is not the target, under
`qemu-user`.

**Second: Windows.** The runtime needs a real shim, not just a sysroot. From the inventory that
the wasm port produced, the POSIX surface that Windows does not have is six things:
`opendir`/`readdir`/`closedir`, `mkdir`, `rmdir`, `unlink`, `stat`, and the `SIGSEGV` fault
handler with its `pthread` stack lookup. The first five are [STDLIB.md](STDLIB.md) sections 5
and 11 and map onto `FindFirstFile`, `_mkdir`, `_rmdir`, `_unlink` and `_stat`.

**The sixth is already solved.** `--depth-guard` ([DECISIONS.md](DECISIONS.md) D030) exists
because WebAssembly has no signals, and Windows has none either in the POSIX sense. A target
that cannot fault-detect gets the counter, which is the same answer, which is the point of
having made it a per-target choice rather than a global one.

**Third: macOS, and not by cross-compiling.** The SDK is not redistributable, so a Linux
machine cannot legally or practically produce a Darwin binary. macOS support means a macOS
machine or a macOS CI runner, and it means the build and test suite running there rather than
here. That is a different kind of work from the other two — infrastructure, not porting — and
the runtime's POSIX use should largely survive it, since Darwin is Unix.

**Not proposed: mobile, desktop, `wasm64`.** [PLATFORMS.md](PLATFORMS.md) lists them as goals.
None has a design, none has a use case, and iOS additionally has the SDK problem plus an
app-store story that is not a compiler problem.

## 3. What has to change in the compiler

Small, and the same for each target:

1. A `Target` variant with its triple and entry symbol. WebAssembly needed
   `__main_argc_argv`; the others use `main`, except that Windows links differently.
2. A runtime object per target in `build.rs`, which already does this three times and skips
   gracefully when a sysroot is missing — the pattern `DOVIK_WASI_SYSROOT` established.
3. A platform shim in `compiler/runtime-c/` for Windows only, behind `#if defined(_WIN32)`,
   the way the fault handler now sits behind `#if !defined(__wasm__)`.
4. A test harness that can *run* the result, which is the part that decides whether the target
   counts as supported ([PLATFORMS.md](PLATFORMS.md) 9). `qemu-user` for aarch64, `wine` or a
   Windows runner for Windows, a macOS runner for macOS.

Step 4 is the one that is not mechanical, and it is the one that matters: a target that builds
but has never run a program is not a target.

## 4. Open decisions

- Whether the project takes on cross sysroots as build dependencies, per target, the way it
  took on `wasi-libc`. Each is a decision like that one.
- Whether a target may be "builds but untested". [PLATFORMS.md](PLATFORMS.md) 9 says no, and
  that rule is the reason this proposal recommends aarch64 first rather than Windows, which is
  the more wanted target and the harder one to test.
- Whether `dovik build --target` should accept an arbitrary triple rather than a short name.
  Today it accepts `native` and `wasm` and rejects everything else, which is honest about what
  has been tested.

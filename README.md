# Dovik

Dovik is an experimental, statically typed programming language designed for simple syntax, automatic type inference, and native compilation.

## Status

**Working, and narrow.** Dovik programs compile to native Linux x86-64 executables and run. The compiler, the C runtime, and a first standard library all exist, under 394 tests.

What is deliberately missing: user-defined types, any target other than Linux x86-64, a project manifest, dependency management, and reclamation of reference cycles. [docs/ROADMAP.md](docs/ROADMAP.md) tracks each one. No published performance claim exists outside [docs/BENCHMARKS.md](docs/BENCHMARKS.md), which carries the environment it was measured in.

The documentation is the contract. Where the implementation disagrees with [docs/SPEC.md](docs/SPEC.md), the implementation is wrong.

## Goals

- Python-like readability with static typing.
- Automatic type inference, with annotations optional for variables and required for function parameters.
- Compilation to native executables, initially for Linux x86-64.
- Automatic memory management with low runtime overhead.
- Diagnostics that are reliable for humans and for AI coding agents.
- Performance claims backed by reproducible benchmarks, never by assertion.

Longer-term targets — other operating systems, WebAssembly, mobile, desktop, cross-compilation, self-hosting — are goals, not capabilities. See [docs/PLATFORMS.md](docs/PLATFORMS.md).

## Example

```dovk
name = "Dovik"

function greet(person: String):
    print("Hello, " + person)

greet(name)
```

Source files use the `.dovk` extension.

## Build and run

Dovik needs a Rust toolchain and `clang`. The compiler has no Rust dependencies; it emits LLVM IR as text and hands it to `clang`, which links it against the runtime with full link-time optimization.

```
cd compiler && cargo build --release && cd ..
./compiler/target/release/dovik build examples/fib.dovk -o fib
./fib 10
```

`dovik run` compiles to a temporary executable and runs it in one step, with the program's own arguments after `--`:

```
./compiler/target/release/dovik run examples/fib.dovk -- 10
```

`dovik check` stops after type checking, `dovik format` prints a formatted file or rewrites it with `--write`, and `dovik test` runs every `test_*` function in its own process. `--json` turns every diagnostic into one JSON object per line. [docs/CLI.md](docs/CLI.md) is the full interface, and [examples/](examples/) is the place to start reading Dovik itself.

## Documentation

[docs/SPEC.md](docs/SPEC.md) is authoritative for language semantics. Where another document disagrees with it, the other document is wrong.

| Document | Contents |
| -------- | -------- |
| [docs/SPEC.md](docs/SPEC.md) | The language: syntax, types, inference, scope, operators, collections, modules, optionals, and errors. Authoritative |
| [docs/VISION.md](docs/VISION.md) | Why Dovik exists, who it is for, and what it will not try to be |
| [docs/DECISIONS.md](docs/DECISIONS.md) | Design decisions, their reasoning, and their trade-offs |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | Compiler structure and the Rust/LLVM implementation |
| [docs/RUNTIME.md](docs/RUNTIME.md) | Runtime responsibilities and memory management |
| [docs/STDLIB.md](docs/STDLIB.md) | The built-in functions and what each one costs |
| [docs/GOTCHAS.md](docs/GOTCHAS.md) | Traps in `compiler/`. Read before changing the pass one describes |
| [docs/CLI.md](docs/CLI.md) | The `dovik` command-line interface |
| [docs/PLATFORMS.md](docs/PLATFORMS.md) | Target platforms, current and planned |
| [docs/BENCHMARKS.md](docs/BENCHMARKS.md) | How performance is measured, and what it measured |
| [docs/ROADMAP.md](docs/ROADMAP.md) | Implementation phases and what each one depends on |

## Project principles

- Define the language before implementing it.
- Keep the language small. A feature needs a stated requirement, not just a precedent in another language.
- Measure performance and memory usage; never claim them.
- Build the compiler one tested feature at a time.
- Expand platform support only after the core is reliable.
- Never describe a planned feature as a working one.

## License

Not decided yet.

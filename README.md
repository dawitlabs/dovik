# Dovik

Dovik is an experimental, statically typed programming language designed for simple syntax, automatic type inference, and native compilation.

## Status

**Language design and planning.** The compiler, runtime, and standard library do not exist yet. Nothing in this repository is implemented.

The documentation describes the contract the first implementation must satisfy. It is not a description of working software.

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

Source files use the `.dovk` extension. This example conforms to the current specification; it cannot be run, because there is no compiler.

## Documentation

[docs/SPEC.md](docs/SPEC.md) is authoritative for language semantics. Where another document disagrees with it, the other document is wrong.

| Document | Contents |
| -------- | -------- |
| [docs/SPEC.md](docs/SPEC.md) | The language: syntax, types, inference, scope, operators, collections, errors, and the open questions that block implementation |
| [docs/VISION.md](docs/VISION.md) | Why Dovik exists, who it is for, and what it will not try to be |
| [docs/DECISIONS.md](docs/DECISIONS.md) | Design decisions, their reasoning, and their trade-offs |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | Compiler structure and the Rust/LLVM implementation plan |
| [docs/RUNTIME.md](docs/RUNTIME.md) | Runtime responsibilities and memory-management goals |
| [docs/CLI.md](docs/CLI.md) | Proposed `dovik` command-line interface |
| [docs/PLATFORMS.md](docs/PLATFORMS.md) | Target platforms, current and planned |
| [docs/BENCHMARKS.md](docs/BENCHMARKS.md) | How performance will be measured |
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

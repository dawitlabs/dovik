# Benchmarks

Reproduces the measurements in [../docs/BENCHMARKS.md](../docs/BENCHMARKS.md).

```
cd compiler && cargo build && cargo build -p dovik-rt && cd ..
python3 bench/run.py
```

`BENCH_REPS` sets the repetition count (default 5). Results are written to
`bench/results.json` and printed as they run.

Each workload in `programs/` exists four times — `.dovk`, `.c`, `.rs`, `.py` —
computing the same answer from the same input. The harness compares their output
and warns if any implementation disagrees, so a faster-looking result cannot come
from doing less work.

The Dovik column is built twice: once normally and once with
`--omit-runtime-checks-for-benchmarking`, which drops the overflow, zero-divisor
and conversion checks. That flag exists only to price those checks. It can produce
wrong answers instead of failing, and nothing but this harness should use it.

Not measured, because the language cannot express the programs yet: file input and
output, reading command-line arguments, and therefore the real-world command-line
program [../docs/BENCHMARKS.md](../docs/BENCHMARKS.md) section 4 asks for.

## Fairness

Each workload must do equivalent work in every language, or the number is worthless.
Two past violations, both caught by measuring instead of assuming:

- `collections` let C use `calloc` and direct indexing where Dovik used a hash map,
  because the keys happened to be dense. C now implements open addressing with the same
  load factor and growth policy.
- `calls` was eliminated entirely by both compilers — about 1 ms for 10 million
  iterations — so it measured process startup. It now accumulates through a function over
  list elements, which neither can fold.

Do not add `noinline` or equivalent annotations to defeat elimination. Dovik has no such
annotation, so forbidding C and Rust to inline while Dovik inlines freely manufactures a
win. Defeat elimination with data the optimizer cannot predict instead.

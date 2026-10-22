# Dovik Benchmarking

**Status:** First measurements recorded — see section 11

Every number in Dovik's documentation must come from this process ([DECISIONS.md](DECISIONS.md) D017). Section 11 holds the first run; `bench/run.py` reproduces it.

## 1. Purpose

Dovik aims to deliver fast execution, low memory usage, and efficient compilation.

Benchmarks will measure these goals objectively and help identify performance regressions.

## 2. Principles

- Measure real performance instead of relying on claims.
- Compare equivalent programs performing equivalent work.
- Use reproducible tests and document the test environment.
- Separate compiler performance from program execution performance.
- Report unfavorable results honestly.
- Optimize based on measurements, not assumptions.

## 3. Performance Categories

### Execution Speed

Measure how long compiled programs take to complete defined workloads.

### Memory Usage

Measure peak memory consumption and, where practical, allocation behavior.

### Compilation Speed

Measure the time required to compile programs from source to executable.

### Binary Size

Measure the size of generated executables, including any required runtime components.

### Startup Time

Measure how quickly a compiled program begins executing.

## 4. Initial Benchmarks

The first benchmark suite should include:

- Integer arithmetic and loops
- Function calls
- Recursion
- String operations
- Collection operations
- File input and output
- Memory allocation and object reclamation
- A small real-world command-line program

Two costs that Dovik's design choices impose deserve their own measurements:

- The checked-arithmetic guarantees: the overflow check on every `+`, `-`, `*`, and negation ([SPEC.md](SPEC.md) 7.2, [DECISIONS.md](DECISIONS.md) D009), and the zero-divisor check on every `/` and `%` including `Float` division ([SPEC.md](SPEC.md) 6.2, [DECISIONS.md](DECISIONS.md) D020). Measure both against an unchecked build to price the correctness guarantees honestly. Float-heavy numeric loops are the workload where the division check is most likely to show.
- String indexing by Unicode scalar value over UTF-8 storage, which is O(n) per index ([SPEC.md](SPEC.md) 10.4). Measure it before resolving U3.

More complex workloads can be added as the language develops.

## 5. Comparisons

Dovik may be compared with C, Rust, and Python.

- Compare optimized native executables for compiled languages.
- Compare equivalent implementations using the same input and output.
- Document compiler versions, optimization settings, and runtime configurations.
- Do not compare results produced under materially different conditions without explaining the differences.

## 6. Measurement Method

Each benchmark should record:

- Hardware and operating system
- Language and compiler versions
- Build configuration
- Input size and workload
- Execution time
- Memory usage, when measured
- Number of repetitions and summary statistics

Use repeated runs to reduce noise. Avoid drawing conclusions from a single measurement.

## 7. Performance Goals

Dovik's long-term goals are:

- Competitive native execution speed
- Low memory overhead
- Fast startup
- Reasonable compilation times
- Small executables where practical

Specific numerical targets will be established after a working compiler and baseline measurements exist.

Dovik must not claim to be universally faster than C before sufficient evidence supports such a claim.

## 8. Automation

The benchmark suite should eventually run through scripts or CI.

Results should be recorded consistently so that changes can be compared over time. Performance regressions should be investigated before introducing strict automated thresholds.

## 9. Open Decisions

- Which benchmark suite should be adopted?
- Which optimization levels should be tested?
- How should automatic memory-reclamation pauses be measured? The measurement method depends on which strategy the runtime adopts, and no strategy has been chosen ([RUNTIME.md](RUNTIME.md) section 8).
- Which results should block a release?
- How should benchmark results be published?

## 10. Status

The strategy below is implemented by `bench/run.py` for execution speed, peak
memory, compilation speed and binary size. Startup time, file I/O and the
real-world command-line program are not measured: Dovik has no file I/O and no
access to arguments, so those programs cannot be written yet.

---

## 11. Results — 2026-10-22

Produced by `bench/run.py`; raw data in `bench/results.json`.

### Environment

- CPU: Intel(R) Core(TM) i5-6300U CPU @ 2.40GHz (4 cores)
- OS: Linux 7.1.9-arch1-2
- Dovik 0.1.0, clang 22.1.8, rustc 1.94.1, CPython 3.13.15
- Builds: Dovik `dovik build` (clang -O2 -flto=full over the emitted IR and the runtime),
  C `clang -O2`, Rust `rustc -O`, CPython unoptimized
- 3 repetitions per measurement; the **minimum** is reported
- Timing and peak memory are measured in separate passes: sampling `VmHWM` perturbs the
  program it watches
- This machine throttles. Medians run 1.3-1.5x the minimum, so a single run can invent a
  result. Re-measure anything that looks new.

### Execution time, milliseconds (lower is better)

| Workload | Dovik | Dovik (no checks) | C | Rust | Python |
| --- | --- | --- | --- | --- | --- |
| `arith` | 20.0 | 26.0 | 15.1 | 15.9 | 4915.1 |
| `recursion` | 2.9 | 2.2 | 1.9 | 2.3 | 71.3 |
| `calls` | 12.1 | 9.1 | 4.4 | 6.9 | 2730.8 |
| `collections` | 34.1 | 24.5 | 19.5 | 31.1 | 303.3 |
| `strings` | 2.7 | 3.1 | 11.2 | 21.1 | 39.1 |
| `string_index` | 3.1 | 4.1 | 0.9 | 1.7 | 24.3 |
| `allocation` | 6.3 | 6.2 | 0.9 | 1.7 | 141.1 |

### Peak memory, MB (lower is better)

| Workload | Dovik | Dovik (no checks) | C | Rust | Python |
| --- | --- | --- | --- | --- | --- |
| `arith` | 2.0 | 2.0 | 1.5 | 2.1 | 11.9 |
| `recursion` | 2.0 | 2.2 | 1.5 | 2.1 | 12.0 |
| `calls` | 2.0 | 2.0 | 1.6 | 2.1 | 12.0 |
| `collections` | 12.9 | 12.9 | 10.5 | 17.4 | 36.4 |
| `strings` | 2.4 | 2.3 | 1.7 | 2.2 | 12.0 |
| `string_index` | 2.0 | 2.0 | 0.6 | 2.1 | 11.9 |
| `allocation` | 2.0 | 2.0 | 0.9 | 2.1 | 11.9 |

### Compilation time and binary size

| Workload | Dovik compile (ms) | Dovik binary (KB) | C compile (ms) | C binary (KB) |
| --- | --- | --- | --- | --- |
| `arith` | 204.8 | 15 | 163.5 | 15 |
| `recursion` | 204.2 | 15 | 130.4 | 15 |
| `calls` | 215.5 | 16 | 158.6 | 15 |
| `collections` | 399.9 | 16 | 249.1 | 15 |
| `strings` | 203.9 | 16 | 212.7 | 15 |
| `string_index` | 196.9 | 16 | 133.6 | 15 |
| `allocation` | 237.7 | 16 | 134.2 | 15 |

### Two benchmarks were measuring the wrong thing

Both are fixed above. They are recorded because the suite's credibility depends on it
more than on any number in it.

**`collections` was comparing a hash map against an array.** The Dovik program builds a
`Dict<Int, Int>` with keys `0..99999`; the C program noticed those keys are dense and
used `calloc` plus direct indexing, doing no hashing at all. That is not equivalent work
([BENCHMARKS.md](BENCHMARKS.md) section 2), and it reported Dovik at 6.8x C. Against an
open-addressing hash map in C with the same load factor and growth policy, dictionary
insertion alone is 19.3 ms against 17.4 ms — 1.11x — and the whole workload is 1.75x.

**`calls` was measuring process startup.** It reported about 1 ms for 10 million
iterations, which is 0.1 ns each: both compilers had proved the loop's result and deleted
it. It now accumulates through a function over values read from a list, which neither can
fold.

Fixing the second one produced a false result in the other direction first. Adding
`noinline` to the C and Rust versions stopped their loops being deleted, but Dovik has no
such annotation, so C and Rust were forbidden to inline while Dovik's call was inlined
freely — and Dovik appeared to win by 1.6x. With the annotations removed and all three
compilers equally free, the honest figure is 2.77x.

### Where Dovik beats C

`strings` builds a 20000-character string one character at a time.

| Implementation | Time |
| --- | --- |
| Dovik, `s = s + "x"` | 2.7 ms |
| C, the equivalent source | 11.2 ms |
| C, hand-rolled capacity-doubling buffer | 0.8 ms |

**Dovik is 4.1x faster than the natural C translation and 3.4x slower than C a programmer
tuned by hand.** At ten times the workload the first gap becomes 3.1 ms against 1593.8 ms,
because the asymptotics differ: Dovik is amortized O(n), naive C is O(n²).

The mechanism is structural. [SPEC.md](SPEC.md) 10.3 makes strings immutable, so
`s = s + "x"` means "produce a new string". The runtime holds a reference count, so it can
see the old string has exactly one owner, know that nobody can observe the difference, and
extend it in place. C has no reference count and cannot know this.

String-keyed dictionary lookup is the second case, for the same reason: an immutable string
can memoize its hash, where C recomputes it on every lookup. Measured separately at 20000
keys and 400000 lookups, Dovik is 15.9 ms against 16.3 ms.

**What this supports:** Dovik's semantics permit optimizations C cannot perform
automatically, and two of them are exploited. **What it does not support:** that Dovik is
faster than C.

### Reading the rest

**Scalar work is 1.3-1.5x C.** `arith` and `recursion` are close; `calls` is 2.8x.

**Checked arithmetic is not free, and the two Dovik builds cannot price it.** The analysis
in `src/opt/range.rs` now elides 11 of the 19 overflow checks in this suite, including every
loop induction variable. The one it cannot elide is the accumulator — `total = total + i % 7`
needs the trip count times the per-iteration increment, which is scalar evolution rather
than interval arithmetic — and one remaining `llvm.sadd.with.overflow` is enough that LLVM
still reports:

```
loop not vectorized: call instruction cannot be vectorized
```

against C vectorizing the same loop at width 2 interleaved 2. Closing `arith` means bounding
that accumulator.

**`allocation` at 7.2x is the largest honest gap.** The workload creates and drops 200000
three-element lists. Each Dovik list carries a reference count, a length and a capacity that
C's `malloc(24)` does not, and pays retain and release it cannot skip.

**`string_index` at 3.4x is specified, not slow.** [SPEC.md](SPEC.md) 10.4 indexes strings by
Unicode scalar over UTF-8 storage, which is O(n) per index; C indexes bytes in O(1) and so
solves an easier problem. This is the measurement U3 asked for.


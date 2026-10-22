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
- 5 repetitions per measurement; the best is reported
- Timing and peak memory are measured in separate passes: sampling `VmHWM` steals CPU
  from the program it watches.

### Execution time, milliseconds (lower is better)

| Workload | Dovik | Dovik (no checks) | C | Rust | Python |
| --- | --- | --- | --- | --- | --- |
| `arith` | 15.0 | 19.7 | 12.1 | 12.2 | 2662.2 |
| `recursion` | 2.1 | 1.7 | 1.6 | 1.7 | 56.6 |
| `calls` | 0.9 | 0.9 | 0.8 | 19.1 | 2785.7 |
| `collections` | 29.8 | 30.1 | 2.5 | 10.2 | 161.2 |
| `strings` | 1.3 | 1.4 | 4.6 | 8.2 | 35.0 |
| `string_index` | 2.7 | 2.5 | 0.9 | 1.4 | 20.8 |
| `allocation` | 8.8 | 7.1 | 0.9 | 1.9 | 123.1 |

### Peak memory, MB (lower is better)

| Workload | Dovik | Dovik (no checks) | C | Rust | Python |
| --- | --- | --- | --- | --- | --- |
| `arith` | 1.9 | 2.0 | 1.5 | 2.1 | 11.9 |
| `recursion` | 2.0 | 2.0 | 1.5 | 2.1 | 12.0 |
| `calls` | 1.0 | 0.8 | 1.1 | 2.1 | 11.9 |
| `collections` | 22.8 | 22.8 | 3.7 | 7.8 | 36.3 |
| `strings` | 2.3 | 1.9 | 1.7 | 2.3 | 12.0 |
| `string_index` | 2.1 | 2.0 | 0.8 | 2.1 | 11.9 |
| `allocation` | 2.0 | 2.0 | 0.7 | 2.1 | 11.9 |

### Compilation time and binary size

| Workload | Dovik compile (ms) | Dovik binary (KB) | C compile (ms) | C binary (KB) |
| --- | --- | --- | --- | --- |
| `arith` | 132.3 | 15 | 87.5 | 15 |
| `recursion` | 140.9 | 15 | 92.2 | 15 |
| `calls` | 122.5 | 15 | 88.0 | 15 |
| `collections` | 169.8 | 16 | 101.5 | 15 |
| `strings` | 199.9 | 16 | 104.0 | 15 |
| `string_index` | 172.6 | 16 | 116.4 | 15 |
| `allocation` | 202.1 | 16 | 129.8 | 15 |

### The one workload where Dovik beats C, stated carefully

`strings` builds a 20000-character string one character at a time. Three versions of
the same program:

| Implementation | Time |
| --- | --- |
| Dovik, `s = s + "x"` | 1.17 ms |
| C, the equivalent source — allocate, copy, free each round | 4.54 ms |
| C, hand-rolled capacity-doubling buffer | 0.78 ms |

**Dovik is 3.9x faster than the natural C translation, and 1.5x slower than C a careful
programmer tuned by hand.** At ten times the workload the first gap becomes 3.1 ms
against 1593.8 ms, because the two have different asymptotics: Dovik is amortized O(n)
and naive C is O(n²).

The reason is structural, not a trick. [SPEC.md](SPEC.md) 10.3 makes strings immutable,
so `s = s + "x"` means "produce a new string". But the runtime holds a reference count,
so it can see that the old string has exactly one owner and no one can observe the
difference — and extend it in place instead of copying. C has no reference counts and
cannot know that, so the equivalent C must copy.

**What this does and does not support.** It supports: Dovik beats C on this workload for
equivalent source, and gives amortized append from code that reads like the naive
version. It does not support: Dovik is faster than C. Hand-tuned C still wins here, and
C wins every other workload in the suite.

The honest general claim is that Dovik's semantics permit optimizations C cannot perform
automatically, and this is one of them being exploited.

### Reading the rest

**Scalar work is within 1.2-1.7x of C.** `arith`, `recursion` and `calls` are close.

**Checked arithmetic is not free, and comparing the two Dovik builds cannot see why.**
LLVM declines to vectorize the checked loop and names the reason:

```
loop not vectorized: call instruction cannot be vectorized
loop not vectorized: value that could not be identified as reduction
                     is used outside the loop
```

`llvm.sadd.with.overflow` is a call the loop vectorizer will not handle. The same loop in
C vectorizes at width 2 interleaved 2. The unchecked build does not recover it either, so
timing the two Dovik builds against each other reports nothing. **Pricing the checks
requires measuring vectorization, not wall time.** Eliding a check that provably cannot
fire would restore it without weakening [DECISIONS.md](DECISIONS.md) D009; that analysis
is not implemented.

**Collections remain the worst gap, around 12x C.** Each element is a 16-byte tagged cell
against C's 8-byte `long long`, which doubles memory traffic and blocks vectorized
summing. Monomorphizing `List<Int>` to a raw array would close most of it.

**String indexing is still O(n) per index, as [SPEC.md](SPEC.md) 10.4 specifies.** It is
35x faster than the first implementation, because the runtime walks UTF-8 continuation
bytes directly, but the asymptotics are unchanged. That is the measurement U3 wanted.

### What the C runtime and pooling changed

| | Rust runtime | C runtime, LTO, in-place append, pooling |
| --- | --- | --- |
| Binary, program printing one number | 5,582,432 bytes | 16,344 bytes |
| `strings` | 172.6 ms | 1.3 ms |
| `string_index` | 235.4 ms | 6.7 ms |
| `allocation` | 62.9 ms | 8.8 ms |
| `collections` | 136.4 ms | 29.8 ms |


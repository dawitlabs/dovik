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

First measurements. Produced by `bench/run.py`; raw data in `bench/results.json`.

### Environment

- CPU: Intel(R) Core(TM) i5-6300U CPU @ 2.40GHz (4 cores)
- OS: Linux 7.1.9-arch1-2
- Dovik 0.1.0, clang 22.1.8, rustc 1.94.1, CPython 3.13.15
- Builds: Dovik `dovik build` (clang -O2 on emitted IR), C `clang -O2`, Rust `rustc -O`, CPython unoptimized
- 5 repetitions per measurement; the best is reported
- Peak memory is absolute `VmHWM`, sampled every 0.5 ms. Runs under ~20 ms under-report.

### Execution time, milliseconds (lower is better)

| Workload | Dovik | Dovik (no checks) | C | Rust | Python |
| --- | --- | --- | --- | --- | --- |
| `arith` | 16.2 | 16.4 | 12.0 | 12.9 | 2977.8 |
| `recursion` | 2.4 | 2.1 | 1.7 | 1.7 | 62.5 |
| `calls` | 1.2 | 1.2 | 0.8 | 19.4 | 3147.6 |
| `collections` | 135.3 | 128.4 | 4.2 | 23.5 | 179.9 |
| `strings` | 18.5 | 22.0 | 4.7 | 7.8 | 33.2 |
| `string_index` | 83.3 | 81.8 | 0.8 | 1.5 | 19.4 |
| `allocation` | 59.9 | 62.4 | 0.9 | 1.5 | 140.4 |

### Peak memory, MB (lower is better)

| Workload | Dovik | Dovik (no checks) | C | Rust | Python |
| --- | --- | --- | --- | --- | --- |
| `arith` | 2.5 | 2.3 | 1.5 | 2.1 | 11.9 |
| `recursion` | 3.0 | 2.3 | 1.8 | 2.1 | 12.0 |
| `calls` | 0.8 | 0.8 | 1.4 | 2.1 | 12.0 |
| `collections` | 16.2 | 14.8 | 4.0 | 7.9 | 36.4 |
| `strings` | 3.2 | 3.2 | 1.7 | 2.3 | 12.0 |
| `string_index` | 2.9 | 2.9 | 1.0 | 2.1 | 11.9 |
| `allocation` | 2.6 | 2.6 | 1.0 | 2.1 | 11.9 |

### Compilation time and binary size

| Workload | Dovik compile (ms) | Dovik binary (KB) | C compile (ms) | C binary (KB) |
| --- | --- | --- | --- | --- |
| `arith` | 310.5 | 5451 | 91.3 | 15 |
| `recursion` | 304.9 | 5451 | 98.9 | 15 |
| `calls` | 307.1 | 5451 | 94.1 | 15 |
| `collections` | 323.7 | 5455 | 102.9 | 15 |
| `strings` | 305.6 | 5455 | 102.0 | 15 |
| `string_index` | 297.4 | 5455 | 105.0 | 15 |
| `allocation` | 310.4 | 5451 | 103.7 | 15 |

### Reading these numbers

**Checked arithmetic is close to free on this workload.** `arith` exists to price it,
and the checked and unchecked builds land within noise of each other. LLVM keeps the
overflow branch off the hot path. The honest caveat is that no workload yet isolates
`Float` division, which section 4 predicts is where the zero-divisor check should show.

**Dovik is within ~1.5x of C on scalar work.** `arith`, `recursion` and `calls` are
competitive. `collections`, `strings` and `string_index` are not, for the two separate
reasons below.

**String indexing is O(n) per index, as specified.** `string_index` performs 2000
indexed reads into a 20000-scalar string. That is [SPEC.md](SPEC.md) 10.4 behaving
exactly as written, and it is the measurement U3 asked for before resolving.

**Collections carry a call per element.** Every `append` and every indexed read crosses
into the runtime as an opaque call, so LLVM can neither inline nor vectorize any of it,
and each element occupies a 16-byte tagged cell against C's 8-byte `long long`. That is
a consequence of the runtime ABI, not of the language.

Nothing here supports a claim that Dovik is fast. It supports the narrower claims that
checked integer arithmetic costs little on this workload and this machine, and that
scalar code is in the same order as C.

### What reference counting changed

These are the same benchmarks before and after the runtime learned to free
([RUNTIME.md](RUNTIME.md) section 3). The earlier runtime never reclaimed anything.

| Workload | Peak memory before | after | Time before | after |
| --- | --- | --- | --- | --- |
| `strings` | 197.1 MB | 3.1 MB | 172.6 ms | 19.4 ms |
| `string_index` | 195.9 MB | 3.2 MB | 235.4 ms | 29.7 ms |
| `allocation` | 23.8 MB | 2.6 MB | 48.6 ms | 62.9 ms |

`strings` got faster because it stopped touching 200 MB, not because the work changed.
`allocation` got slower, which is the cost showing up where it should: that workload
creates and drops 200000 short-lived lists, so it pays retain and release on every one
and gains nothing, since it was never memory-bound. Reclamation is not free, and this
is the workload that proves it.


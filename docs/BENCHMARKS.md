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
- Builds: Dovik `dovik build` (clang -O2 on emitted IR), C `clang -O2`, Rust `rustc -O`, Python unoptimized
- 5 repetitions per measurement; the best is reported
- Peak memory is absolute `VmHWM`, sampled every 0.5 ms. Runs under ~20 ms under-report.

### Execution time, milliseconds (lower is better)

| Workload | Dovik | Dovik (no checks) | C | Rust | Python |
| --- | --- | --- | --- | --- | --- |
| `arith` | 33.6 | 33.3 | 25.5 | 27.4 | 3822.9 |
| `recursion` | 5.2 | 7.0 | 9.1 | 13.4 | 126.1 |
| `calls` | 1.3 | 1.5 | 1.0 | 24.5 | 3670.3 |
| `collections` | 134.8 | 123.1 | 2.7 | 10.8 | 170.1 |
| `strings` | 172.6 | 188.0 | 5.0 | 9.0 | 35.5 |
| `string_index` | 235.4 | 244.4 | 1.0 | 1.6 | 20.3 |
| `allocation` | 62.4 | 58.3 | 1.1 | 2.4 | 158.4 |

### Peak memory, MB (lower is better)

| Workload | Dovik | Dovik (no checks) | C | Rust | Python |
| --- | --- | --- | --- | --- | --- |
| `arith` | 2.5 | 2.4 | 1.5 | 17.4 | 17.4 |
| `recursion` | 3.0 | 17.4 | 1.8 | 17.4 | 12.0 |
| `calls` | 1.4 | 0.8 | 1.3 | 2.1 | 12.0 |
| `collections` | 14.8 | 14.8 | 3.6 | 7.8 | 36.4 |
| `strings` | 197.1 | 197.2 | 1.7 | 2.4 | 12.0 |
| `string_index` | 195.9 | 196.1 | 1.3 | 2.1 | 11.9 |
| `allocation` | 23.8 | 23.7 | 1.4 | 17.4 | 11.9 |

### Compilation time and binary size

| Workload | Dovik compile (ms) | Dovik binary (KB) | C compile (ms) | C binary (KB) |
| --- | --- | --- | --- | --- |
| `arith` | 320.3 | 5417 | 287.8 | 15 |
| `recursion` | 557.4 | 5417 | 308.2 | 15 |
| `calls` | 537.8 | 5417 | 282.1 | 15 |
| `collections` | 302.3 | 5418 | 131.8 | 15 |
| `strings` | 322.4 | 5417 | 107.5 | 15 |
| `string_index` | 372.2 | 5418 | 104.4 | 15 |
| `allocation` | 317.6 | 5417 | 92.1 | 15 |

### Reading these numbers

**Checked arithmetic is close to free here.** `arith` is the workload built to
price it, and the checked and unchecked builds land within noise of each other on
this machine. LLVM keeps the overflow branch off the hot path. The one honest
caveat is that no workload yet isolates `Float` division, which section 4 predicts
is where the zero-divisor check should show.

**Dovik is within ~2x of C on scalar work and far off it elsewhere.** `arith`,
`recursion` and `calls` are competitive. `collections`, `strings` and
`string_index` are not, for two separate reasons below.

**Memory is the headline problem.** `strings` peaks at over 200 MB against C's
1.7 MB. The native runtime never frees ([RUNTIME.md](RUNTIME.md) has not chosen a
reclamation strategy), so every intermediate string from 20000 concatenations is
still live at exit. `allocation` shows the same effect more mildly. This is the
clearest argument in the suite for settling the memory question.

**String indexing is O(n) per index, as specified.** `string_index` performs 2000
indexed reads into a 20000-scalar string and costs ~230 ms against C's 1 ms. That
is [SPEC.md](SPEC.md) 10.4 behaving exactly as written, and it is the measurement
U3 asked for before resolving.

**Collections carry a function-call cost per element.** Every `append` and every
indexed read crosses into the runtime as an opaque call, so LLVM cannot inline or
vectorize any of it. That is a consequence of the tagged-cell ABI, not of the
language.

Nothing here supports any claim that Dovik is fast. It supports the narrower claim
that checked integer arithmetic costs little on this workload and this machine.

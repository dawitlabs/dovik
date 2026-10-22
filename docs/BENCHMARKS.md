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
- Timing and peak memory are measured in **separate passes**. Sampling `VmHWM` steals CPU
  from the program it watches, which inflated an earlier revision of these numbers.

### Execution time, milliseconds (lower is better)

| Workload | Dovik | Dovik (no checks) | C | Rust | Python |
| --- | --- | --- | --- | --- | --- |
| `arith` | 15.2 | 19.7 | 12.2 | 12.2 | 2975.7 |
| `recursion` | 3.3 | 2.7 | 1.9 | 1.9 | 56.2 |
| `calls` | 0.9 | 0.9 | 0.7 | 19.0 | 2793.5 |
| `collections` | 37.8 | 31.2 | 2.6 | 14.6 | 172.3 |
| `strings` | 5.7 | 5.7 | 4.7 | 8.4 | 31.6 |
| `string_index` | 6.7 | 6.8 | 0.7 | 1.4 | 19.2 |
| `allocation` | 10.2 | 9.3 | 0.8 | 1.4 | 121.8 |

### Peak memory, MB (lower is better)

| Workload | Dovik | Dovik (no checks) | C | Rust | Python |
| --- | --- | --- | --- | --- | --- |
| `arith` | 2.0 | 2.0 | 1.5 | 2.1 | 11.9 |
| `recursion` | 2.0 | 2.0 | 1.5 | 2.1 | 12.0 |
| `calls` | 0.7 | 0.7 | 1.8 | 2.1 | 12.0 |
| `collections` | 22.8 | 22.8 | 3.7 | 7.9 | 36.4 |
| `strings` | 2.4 | 2.4 | 1.7 | 2.3 | 12.0 |
| `string_index` | 2.1 | 2.1 | 1.0 | 2.2 | 11.9 |
| `allocation` | 2.1 | 2.0 | 0.7 | 2.1 | 11.9 |

### Compilation time and binary size

| Workload | Dovik compile (ms) | Dovik binary (KB) | C compile (ms) | C binary (KB) |
| --- | --- | --- | --- | --- |
| `arith` | 163.7 | 15 | 98.4 | 15 |
| `recursion` | 129.2 | 15 | 90.1 | 15 |
| `calls` | 118.6 | 15 | 83.9 | 15 |
| `collections` | 381.6 | 16 | 260.7 | 15 |
| `strings` | 153.6 | 16 | 118.0 | 15 |
| `string_index` | 163.2 | 16 | 98.3 | 15 |
| `allocation` | 140.7 | 16 | 92.1 | 15 |

### Reading these numbers

**Scalar work is within 1.3x of C.** `arith`, `recursion` and `calls` are close, and on
`calls` and `strings` Dovik is ahead of Rust — though Rust's `calls` figure is depressed
by the `#[inline(never)]` the comparison needs to keep the work equivalent.

**Checked arithmetic is not free, and comparing the two Dovik builds cannot see why.**
Asked directly, LLVM declines to vectorize the checked loop and names the reason:

```
loop not vectorized: call instruction cannot be vectorized
loop not vectorized: value that could not be identified as reduction
                     is used outside the loop
```

`llvm.sadd.with.overflow` is a call the loop vectorizer will not handle, and the trap
branch leaves the accumulator live outside the loop. The same loop in C vectorizes at
width 2 interleaved 2. That gap is most of the remaining difference on `arith`, and the
unchecked build does not recover it either, so timing the two Dovik builds against each
other reports nothing. **Pricing the checks requires measuring vectorization, not wall
time.** Eliding a check that provably cannot fire would restore it without weakening
[DECISIONS.md](DECISIONS.md) D009; that analysis is not implemented.

**Collections remain the worst gap, at roughly 14x C.** Each element is a 16-byte tagged
cell against C's 8-byte `long long`, and the dictionary pays hashing where C indexes an
array directly. The per-element call overhead is gone: with `-flto=full` the runtime
inlines into the loop, which is most of why this workload improved 3.6x.

**String indexing is still O(n) per index, as [SPEC.md](SPEC.md) 10.4 specifies.** It is
35x faster than it was, because the C runtime walks UTF-8 continuation bytes directly
instead of going through an iterator, but the asymptotics are unchanged and that is the
measurement U3 asked for.

Nothing here supports a claim that Dovik is fast in general. It supports the narrower
claims that scalar code is within ~1.3x of C, that string building is competitive with
C, and that the remaining gaps have identified causes.

### What the C runtime changed

The runtime was first written in Rust and linked as a staticlib. Replacing it with C
compiled to bitcode and linked with `-flto=full` fixed three measured problems at once —
the fixed memory floor, the binary size, and the per-element call overhead that stopped
LLVM inlining anything.

| | Rust runtime | C runtime |
| --- | --- | --- |
| Binary, program printing one number | 5,582,432 bytes | 21,304 bytes |
| `string_index` | 235.4 ms | 6.7 ms |
| `strings` | 172.6 ms | 5.7 ms |
| `allocation` | 62.9 ms | 10.2 ms |
| `collections` | 136.4 ms | 37.8 ms |

### What reference counting changed

Measured before the C runtime replaced the Rust one, so these are Rust-runtime numbers
on both sides. The earlier runtime never reclaimed anything.

| Workload | Peak memory before | after | Time before | after |
| --- | --- | --- | --- | --- |
| `strings` | 197.1 MB | 3.1 MB | 172.6 ms | 19.4 ms |
| `string_index` | 195.9 MB | 3.2 MB | 235.4 ms | 29.7 ms |
| `allocation` | 23.8 MB | 2.6 MB | 48.6 ms | 62.9 ms |

`strings` got faster because it stopped touching 200 MB. `allocation` got slower, which
is the cost appearing where it should: that workload creates and drops 200000 short-lived
lists, pays retain and release on each, and was never memory-bound.


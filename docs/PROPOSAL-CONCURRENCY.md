# Proposal: Concurrency, and What It Costs Before It Exists

**Status:** **Draft. Recommending that Dovik not add concurrency yet, with a measurement
rather than an opinion.** Nothing here is implemented. [SPEC.md](SPEC.md) section 16 lists
concurrency as absent, and this file is the design record for why it stays that way for now.

## 1. The one measurement that matters

[RUNTIME.md](RUNTIME.md) section 11 says reference counts are non-atomic "because Dovik has no
concurrency. Adding threads invalidates that and nothing else here." The second sentence is
true and the first hides a price, so the price was measured.

A second runtime was built from the same source with two lines changed — `header->count += 1`
became `__atomic_fetch_add(..., __ATOMIC_RELAXED)` and `--header->count > 0` became
`__atomic_sub_fetch(..., __ATOMIC_ACQ_REL) > 0` — and both binaries were built first and then
run strictly alternately, fifteen rounds each, keeping the minimum.

| Workload | Non-atomic | Atomic | Cost | Reference counting? |
| --- | --- | --- | --- | --- |
| allocation | 4.11 ms | 7.16 ms | **+74%** | yes |
| strings | 1.61 ms | 2.48 ms | **+54%** | yes |
| string_index | 1.28 ms | 1.81 ms | +41% | yes |
| collections | 14.71 ms | 14.85 ms | +1% | yes |
| recursion | 2.29 ms | 2.30 ms | +0.4% | no — control |
| calls | 11.38 ms | 9.50 ms | −17% | no — control |
| arith | 27.13 ms | 22.43 ms | −17% | no — control |

**Read the controls first.** `arith` and `calls` do no reference counting at all, so atomics
cannot make them faster; the −17% is code layout under full LTO differing between two binaries,
and it sets the noise floor for this experiment at about 17%. That makes the `allocation` and
`strings` numbers real, `string_index` suggestive, and `collections` genuinely free — it spends
its time inside dictionary and list operations rather than on the retain and release path.

So: **atomic counts cost up to about 74% on allocation-churn code, and nothing on code that
does not churn.** That cost would be paid by every program, including the single-threaded ones,
because one runtime is linked into all of them.

## 2. What that implies

Three ways out, in increasing order of how much they change the language.

**Pay it.** Simple, one runtime, and it makes the `allocation` benchmark — already the largest
honest gap against C at 6.0x ([BENCHMARKS.md](BENCHMARKS.md) 11) — roughly 10x. That trades the
project's stated performance goal for a feature nobody has asked for.

**Build two runtimes.** The build already produces three objects — native, wasm, sanitized — so
a fourth is mechanical. But then a program's performance depends on whether it *might* use
threads, which is a cliff rather than a cost, and a library compiled one way cannot link the
other. Dovik has no libraries yet, which is the only reason this is even tractable.

**Change what is shared.** If values cannot cross a thread boundary at all, counts stay
non-atomic and the cost is zero. That is the message-passing design: a channel moves a value,
the sender loses it, and nothing is ever counted from two threads. It is also the design that
needs the most new language — ownership transfer is a type-system concept, and
[SPEC.md](SPEC.md) 3.4 currently has exactly two categories, value and reference, with no third
for "moved".

## 3. Why not yet

**Nothing in the repository wants a thread.** The compiler is single-threaded. The three
examples are command-line filters. The benchmark suite measures one process. Adding concurrency
now means designing a memory model, a synchronization story, and a cancellation story against
zero real programs, and [SPEC.md](SPEC.md) section 20 is explicit that the rule changes before
the implementation does.

**It is coupled to the question Dovik deliberately deferred.** Shared mutable state across
threads makes reference cycles constructible in a way the type system currently prevents
([RUNTIME.md](RUNTIME.md) 11, and `no_value_can_be_made_to_reach_itself` in `tests/check.rs`
holds that). A thread-safe Dovik needs the cycle collector that a single-threaded Dovik proved
it does not.

**The thing people actually want from concurrency here is I/O, not parallelism.** A web server
wants to wait on many sockets, not to use many cores. That is
[PROPOSAL-NETWORKING.md](PROPOSAL-NETWORKING.md)'s problem, and it has a cheaper answer there
than threads.

## 4. The trigger

Write this design when a real Dovik program is limited by one core, and say which program and
by how much. Until then the honest state is: no concurrency, non-atomic counts, and a measured
number for what changing that would cost.

## 5. Open decisions

- Whether message passing or shared memory, if it ever happens. Section 2 prefers message
  passing on cost grounds alone, which is not the same as having designed it.
- Whether a second runtime is acceptable, and what happens to a library compiled against one.
- Whether `sort`, `append` and the other mutating builtins get documented thread behaviour or
  stay undefined. Today the runtime "promises no thread safety for mutable collections"
  ([RUNTIME.md](RUNTIME.md) 8), which is honest because nothing can observe it.

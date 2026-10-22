# Compiler Implementation Gotchas

Traps and non-obvious constraints in `compiler/`. Read before changing the pass
they describe. Language semantics live in [SPEC.md](SPEC.md); this file is only
about the implementation.

## Runtime checks are inside their instruction, not beside it

Every failure in [SPEC.md](SPEC.md) 15.2 is a property of the IR instruction
that carries it. `IntArith` *is* checked addition; there is no unchecked variant
to swap it for. This is deliberate: [ARCHITECTURE.md](ARCHITECTURE.md) section 4
requires that no later pass can discard a required check, and a check expressed
as a separate compare-and-branch is something an optimizer can prove away.

`InstKind::runtime_error()` reports which failure an instruction can raise, and
`Program::checks()` enumerates them. Do not add an unchecked arithmetic
instruction without a verification pass that proves the check moved rather than
vanished.

## Integer arithmetic is checked, float arithmetic is not

[SPEC.md](SPEC.md) 7.3: float arithmetic fails in exactly one case, division by
zero. Overflow to infinity and NaN are legal results. So `FloatArith` carries no
check and `IntArith` does. A constant fold that produces infinity is correct and
must not be rejected; a constant fold that overflows an `Int`, divides by zero,
or takes a remainder by zero must be a compile error.

## The optimizer propagates constants forward, and only within a block

An earlier version substituted folded constants *backwards* into already-visited
instructions, so consumers kept reading registers whose defining instruction had
been deleted and saw zero. `print(7 / 2)` reported division by zero.

Two rules follow:

1. Propagate forward. Rewrite an instruction's operands from the known-constant
   map before attempting to fold it.
2. A register defined in one block can be read in another — short-circuit
   lowering does exactly this. `opt::escape` finds those registers, and a folded
   definition is only deleted when every read is in its own block.

## Reference cycles are not constructible, so `Rc` is sufficient

[RUNTIME.md](RUNTIME.md) lists cyclic reference handling as open, and
[SPEC.md](SPEC.md) section 14 requires cycles be reclaimable, which rules out
plain reference counting in general.

It does not apply to the initial language. Types are `Int`, `Float`, `Bool`,
`String`, `List<T>` and `Dict<K, V>`, with no user-defined or recursive types. A
cycle needs `T` to transitively contain `List<T>`, which needs an infinitely
nested type annotation. No program can write one, so the interpreter's
`Rc<RefCell<..>>` leaks nothing. Introducing recursive types, or any named type,
invalidates this and brings the cycle problem back.

## The interpreter is a test oracle, not a product

[DECISIONS.md](DECISIONS.md) D016 rules out shipping an interpreter or VM.
`src/interp` exists so the frontend can be executed and the optimizer can be
checked against unoptimized output; `tests/opt.rs` runs a corpus both ways and
compares. It is not a supported way to run Dovik programs and must not appear in
user-facing documentation as one.

Its step and recursion limits are not language semantics.
[SPEC.md](SPEC.md) 15.2 leaves stack exhaustion unspecified (U7); the limits only
stop a test from hanging.

## Parser recovery must not consume the next line

A statement can report its error *after* consuming its own line break — `1 = 2`
validates its assignment target last. Recovery then sees a cursor already parked
on the next statement, and skipping forward would discard a good line along with
any error it would have reported.

`Parser::synchronize` checks whether the previous token was a line break, indent
or dedent, and does nothing if so. Every error path depends on that guard, not
just the ones that exist today.

## An unclaimed indent means the block header already failed

When a block header fails to parse, the lexer has still emitted `Indent` and
`Dedent` around the body. Reporting those at statement position produces two
extra errors for one mistake. The parser skips the whole balanced region
silently, and only reports when no error has been recorded yet, so a genuinely
stray indent cannot disappear.

## `Unknown` suppresses cascades, and must stay transitive

`Type::Unknown` matches any type at any depth. Every type comparison goes
through `Type::matches`, never `==`, so one bad expression reports once instead
of again at every use. A rejected annotation is also returned as `Unknown` for
the same reason.

Comparing types with `==` anywhere in `src/check` reintroduces the cascades.

## Scalar iteration over a string must not re-index

[SPEC.md](SPEC.md) 10.4: indexing a string by scalar position is O(n). Lowering
`for` to repeated `string.index` would make the loop quadratic, which is the
exact thing the spec tells programmers to use `for` to avoid. The lowering walks
a byte cursor through `string.next` instead.

## Open question: how large floats print

`print` formats a `Float` as the shortest decimal that round-trips
([SPEC.md](SPEC.md) 6.7). Rust's formatter never uses exponent notation, so
`1e152` prints as 153 digits. That is consistent with Dovik having no exponent
literal syntax (2.8), but it is an observable output decision the spec does not
settle.

## The backend emits LLVM IR as text, not through bindings

[ARCHITECTURE.md](ARCHITECTURE.md) section 7 lists "LLVM version and choice of
Rust bindings" as open. The backend does not need that decision: it writes
textual LLVM IR and shells out to `clang`, so there is no compile-time version
pin and no `inkwell` or `llvm-sys` dependency. [DECISIONS.md](DECISIONS.md) D002
calls for LLVM, which this satisfies — it does not call for in-process bindings.

`inkwell` and `llvm-sys` historically lag LLVM releases by several versions, so
on a current toolchain the textual route may be the only one available. Verify
their support before reopening this.

## Checked arithmetic is emitted inline, not as a runtime call

Overflow uses `llvm.sadd/ssub/smul.with.overflow.i64` and branches to a block
that calls `dovik_trap`. Division and remainder compare the divisor against zero
and branch the same way. `to_int` checks NaN, both infinities and the `Int` range
after `llvm.trunc.f64`.

LLVM cannot remove those branches unless it can prove the operation cannot fail,
which is the behaviour we want. `tests/native.rs` compiles eight trapping
programs at `clang -O2` and asserts each one still aborts; that test is the
guarantee, not the emitter.

## `srem` is undefined for `Int::MIN % -1`, so the divisor is rewritten

LLVM's `srem` is undefined behaviour when the dividend is `Int::MIN` and the
divisor is `-1`, even though [SPEC.md](SPEC.md) 6.3 gives that case the perfectly
ordinary answer `0`. The backend selects `1` in place of a `-1` divisor: `x % 1`
and `x % -1` are both `0` for every `x`, and `srem` by `1` is always defined.

Do not simplify this away.

## A string literal becomes a fresh runtime string at every evaluation

`@str.N` is a byte array in `.rodata`. Dovik strings are opaque runtime objects,
so every use of a literal calls `dovik_str_new`. Values therefore come in two
forms and must not be confused: `Emitter::native` yields a value in its own LLVM
type, and `Emitter::as_word` yields the tagged 64-bit word the runtime ABI takes.
Passing `@str.N` where a word belongs is rejected by `llc` with "global variable
reference must have pointer type".

## The native runtime never frees

Every heap value is leaked. [SPEC.md](SPEC.md) section 14 does not specify when
collection runs or guarantee reclamation before exit, and the collector is not
observable, so this conforms — but it is a known gap, not a design, and it is the
first thing to fix when [RUNTIME.md](RUNTIME.md)'s memory-management decision is
made.

## Runtime values are tagged cells, so the backend needs no type information

A runtime value is `{ tag: u8, word: u64 }`. The backend knows every static type
and passes the matching tag; the runtime uses the tag to print and compare
recursively, which is what lets a nested `List<List<Int>>` compare structurally
without the backend describing its shape.

Dictionary keys are normalized before hashing: a `String` key is replaced by a
hash of its bytes so that equal strings are the same key. Add a key type and this
must be extended, or [SPEC.md](SPEC.md) 12.2's "hashing consistent with equality"
breaks.

## Building an executable needs the runtime archive

`-o` links against `libdovik_rt.a`, found next to the compiler binary or at
`DOVIK_RUNTIME`. `cargo build -p dovik-rt` produces it. `tests/native.rs` builds
it on demand rather than skipping, so a missing archive fails loudly instead of
quietly passing.

## Error codes are stable identifiers, not line numbers

Every diagnostic carries a `DVK` code ([SPEC.md](SPEC.md) 15.3), assigned per
error-kind variant in `src/error/*.rs` and `RuntimeError::code`. The prefix is the
pass: 1 lexical, 2 syntax, 3 names, 4 types, 5 runtime.

Codes are part of the CLI's contract with tools that parse its output. Adding a
variant takes the next free number in its range. Never renumber one, and never
reuse a retired number for a different error — delete the variant and leave the
number spent.

The numeric codes passed to `dovik_trap` are a separate, internal sequence that
the runtime maps back to `DVK5xxx`. Changing one without the other makes the
runtime report the wrong error.

## `dovik run` compiles; it does not interpret

[DECISIONS.md](DECISIONS.md) D016 rules out shipping an interpreter, so `run`
builds a real executable to a temporary path, executes it, forwards its exit code
and deletes it. An earlier CLI exposed the IR interpreter as `--run`, which
contradicted D016; the interpreter is reachable only as a library, from tests.

Do not reintroduce a user-facing flag that interprets.

## `>=` closing a type parameter list is a lexical ambiguity

`names: List<String>= []` lexes `>=` as one token, so the type parameter list never
closes. [SPEC.md](SPEC.md) 3.1 argues `<` and `>` in a type are unambiguous because
Dovik has no shift operator, which disposes of `>>` but not of `>=`.

`Parser::eat_type_close` splits the token: it accepts `GtEq` where a type expects
`>`, and leaves a synthetic `=` at the second half of the span for the next read.
The formatter resolves the same ambiguity independently, from its own type-context
tracking, because it works on tokens and never consults the parser.

Two places therefore know this rule. Changing one without the other makes
`dovik format` emit source that no longer parses.

## The formatter cannot repair indentation

Indentation is semantic ([SPEC.md](SPEC.md) 2.4) and a width that is not a multiple
of four is a lexical error. Six spaces could mean one level or two, and nothing in
the file says which, so there is no intended structure to recover. `dovik format`
normalizes spacing, type-name casing, comment spacing and blank runs in a file that
already lexes and parses; it is not a repair tool.

It also refuses to format a file that does not parse, so it can never turn broken
source into differently broken source.

## The formatter works on tokens, not the AST

Comments and blank lines are not in the AST, so an AST-driven formatter would
delete them. `lexer::lex_with_trivia` returns comment spans alongside the tokens,
and blank lines are recovered by comparing source line numbers between logical
lines — the lexer emits no token for a blank line at all.

`tests/format.rs` asserts that formatting a program cannot change what it prints.
That test is the reason to trust the token approach; keep it passing.

## Built-ins are a table, not a list of string comparisons

`src/builtins.rs` holds the name list, the `Builtin` enum, arity and result types.
The resolver seeds the built-in scope from `NAMES`, so adding a function there makes
it visible everywhere at once. Adding one means touching, in order: `builtins.rs`,
`check/builtin.rs` for argument rules, `lower/call.rs`, `interp/builtin.rs`,
`backend/builtin.rs` for the LLVM call, the `declare` lines in `backend/mod.rs`, and
`runtime/src/lib.rs`.

Miss the backend `declare` and `llc` rejects the module; miss the interpreter and the
differential tests in `tests/stdlib.rs` fail. Both failures are loud, which is why
there is no central registry beyond the table.

`print`, `len`, `to_int` and `to_float` keep their own dedicated IR instructions from
before the table existed. Everything else lowers to `InstKind::Builtin`.

## A failed parse reuses the invalid-conversion error

`parse_int` and `parse_float` trap with `RuntimeError::InvalidConversion` (`DVK5004`),
the error `to_int` already uses, rather than introducing a seventh runtime error.
[SPEC.md](SPEC.md) 15.2's list is normative and adding to it is a language change, so
the message was widened instead of the list.

If 15.2 later gains a parse error, the code must change in two places that have to
agree: `RuntimeError::code` in `src/ir/mod.rs` and `code_name` in the runtime.

## Retain and release placement is verified by a test, not by reasoning

`src/backend/ownership.rs` decides which registers hold an owned reference. Producers
that return a fresh object are owned; `LoadLocal`, `LoadGlobal`, `ListGet` and
`DictGet` return borrowed. Every owned register is released exactly once, after its
last use in its own block.

The rule the emitter relies on: **an owned reference register never outlives its
block.** `lower_for` is the reason that holds — it stores the sequence it iterates into
a slot and reloads it, because otherwise the list would be produced in the pre-header
and used in the loop body, and releasing it in the pre-header would free it before the
first iteration. Any new lowering that lets an owned reference cross a block boundary
reintroduces that bug.

Two over-releases found this way, both of which crashed rather than leaked:

- Function parameters are stored into their slot by a raw `store` in the entry block,
  not through `StoreLocal`, so they skip its retain. `release_frame` releases every
  reference slot at exit, so a `String` parameter was freed while the caller still
  owned it. The entry block now retains reference parameters explicitly.
- A dictionary's `String` key is stored as a hash, not a pointer. Releasing a dict
  dereferenced that hash as an address. Hashed keys carry `TAG_HASHED`, which
  `dovik_retain` and `dovik_release` ignore, and dict teardown releases only values.

`DOVIK_CHECK_LEAKS=1` makes the runtime print its live-object count at exit;
`tests/native.rs` asserts it is zero. Keep that test passing and both failure modes
stay caught.

## String literals are allocated once, not per use

`@str.N` is a `.rodata` byte array. The runtime string for each literal is created once
at the top of `main` and stored in `@slit.N`, and every use loads that. Creating one per
use would allocate inside loops, and each would be an owned temporary needing its own
release.

Literals are therefore borrowed everywhere, and live for the whole program. `main`
releases them before reporting leaks, so they do not show up as a leak.

## The overflow check blocks SIMD, which is the real cost of D009

`llvm.sadd.with.overflow` is a call the loop vectorizer refuses, and the trap branch
leaves the accumulator live outside the loop. Asked with
`clang -Rpass-analysis=loop-vectorize`, LLVM says exactly that:

```
loop not vectorized: call instruction cannot be vectorized
loop not vectorized: value that could not be identified as reduction
                     is used outside the loop
```

The same loop in C vectorizes at width 2 interleaved 2. That gap, not the cost of a
branch, is most of why Dovik's integer loops are slower.

**Do not price the checks by comparing the checked and unchecked builds.** Neither
vectorizes, so the comparison reports no difference and looks like the checks are free.
Measure vectorization, not just wall time. `--omit-runtime-checks-for-benchmarking`
removes the intrinsic but something else still prevents vectorization in that build,
and what that is has not been diagnosed.

## Most of a small program's memory is the runtime, not the program

A Dovik program printing one number: 5.58 MB binary, 2.98 MB peak RSS. The same in C:
16 KB and 1.34 MB. `libdovik_rt.a` statically links Rust's std.

Every memory measurement carries that ~3 MB floor, so a workload reporting 3.1 MB is
using about 0.1 MB of its own. Before concluding anything about allocation behaviour
from a memory number, subtract the floor — and know that shrinking the runtime is worth
more than tuning the allocator.

## The runtime is C, compiled to bitcode, linked with full LTO

`runtime-c/dovik_rt.c` is built by `build.rs` with `clang -O2 -flto` into `OUT_DIR`, and
the path is baked into the compiler as `DOVIK_RUNTIME_OBJECT`. Programs link with
`-flto=full`, which lets LLVM inline runtime calls into the hot loop.

**`-flto=full`, not `-flto`.** Plain `-flto` selects thin LTO here and leaves
`dovik_list_push`, `dovik_list_get` and the rest as calls; checking the final binary with
`objdump | grep 'call.*dovik_'` shows them. Full LTO inlines them, which is most of why
collection work got 3.6x faster.

An earlier runtime was written in Rust and linked as a staticlib. It cost 5.58 MB of
binary and ~3 MB of resident memory for a program that prints one number, because it
pulled in Rust's standard library, and it blocked all inlining. The C version is 21 KB
total. Do not reintroduce a Rust-std dependency in the native runtime.

`DOVIK_RUNTIME` overrides the object path. Setting it to a stale artifact silently
measures the wrong runtime — that mistake made the benchmark harness report collection
work as 120 ms when the real figure was 38 ms.

## Benchmark timing and memory must be measured in separate passes

Sampling `/proc/<pid>/status` for `VmHWM` perturbs the process being watched. Measuring
both at once inflated every timing in an earlier revision of the results. `bench/run.py`
now runs the timing repetitions first and the memory passes afterwards.

## In-place string append is only safe because of three guards

`dovik_str_concat` extends its left operand in place instead of copying, which is what
makes `s = s + "x"` amortized O(1) and beats the equivalent C. [SPEC.md](SPEC.md) 10.3
says strings are immutable, so this is only legal while no one can observe it. Three
conditions enforce that, and removing any one is a correctness bug:

1. **`count == 1`.** Exactly one owner, so no other binding can see the change. A string
   also held in a list, a dict or another variable has a higher count and takes the
   copying path.
2. **`frozen == 0`.** A literal is created once at program start and shared by every use
   (`@slit.N`), so its count is 1 while being referenced from many places. `dovik_str_new`
   marks it frozen. Without this, `"p" + n` in a loop would corrupt the literal `"p"`.
3. **`a != b`.** `s = s + s` would read from the buffer it is reallocating.

The in-place path returns the same pointer with the count incremented, because the
backend treats a concat result as owned and will release it once. Returning it without
that increment frees a live string.

`tests/native.rs` covers all three guards — shared variable, self-concat, string in a
list, string in a dict, named literal, returned string, string parameter — by comparing
native output against the interpreter, which has no in-place path. A broken guard shows
up as a mismatch there.

## The object pool bounds itself deliberately

Released strings and lists go onto free lists rather than back to `malloc`, which is most
of why allocation churn improved. Both are capped at 2048 entries and only accept objects
whose buffer is at most 4096 bytes, so a program that briefly builds something huge does
not keep that memory forever.

A pooled object keeps its buffer and capacity; reuse resets `count`, `len` and `frozen`.
Forgetting to reset `frozen` would let a recycled object inherit a literal's immutability
and silently disable in-place append.

Pooling does not affect the leak counter: `live_objects` is decremented on release
whether or not the memory is recycled, so `DOVIK_CHECK_LEAKS` still measures ownership
rather than allocator behaviour.

## Immutable strings memoize their hash; mutation must invalidate it

A `Str` caches its FNV hash after the first dictionary operation that needs it. This is
only sound because [SPEC.md](SPEC.md) 10.3 makes strings immutable — the bytes cannot
change under the cache.

Except through in-place append, which is the one place the bytes do change.
`dovik_str_concat` therefore clears `hashed` on the in-place path. Forgetting that
returns stale hashes and silently loses dictionary entries.

Equality also short-circuits: identical pointers are equal, differing lengths are not,
and two memoized hashes that differ settle it without a `memcmp`. Literals are shared
from one global per literal, so the pointer path fires often in real code.

## The dictionary is one array of entries, for the cache

Keys, values and occupancy were three parallel arrays, which cost three cache misses per
probe and made Dovik slower than C at dictionary lookup despite C recomputing its hash
every time. They are now one `Entry` array, and `TAG_EMPTY` in a key's tag marks a free
slot so no separate occupancy array is needed.

Nothing may store a real value with tag 255.

## Containers are monomorphic, so element tags live in the header

A `List<T>` holds only `T` and a `Dict<K, V>` only `K` and `V`, so a per-element tag was
pure waste. `List` stores `elem` once and its items are raw 8-byte words; `Dict` stores
`key_tag` and `val_tag` once and its entries are two raw words. That halved memory
traffic and is most of why collection work went from 29.8 ms to 17.0 ms.

Consequences to respect:

- `dovik_list_new` takes the element tag and `dovik_dict_new` takes both. A container
  created with the wrong tag will retain and release the wrong things.
- Retain and release are skipped entirely when the tag is a scalar (`< TAG_STR`), which is
  why scalar lists cost nothing to drop.
- List equality uses `memcmp` for `Int` and `Bool` only. **Not for `Float`:**
  [SPEC.md](SPEC.md) 13 requires NaN to differ from itself and -0.0 to equal 0.0, and bit
  comparison gets both backwards.
- `Dict` keeps occupancy in a separate `state` byte array allocated in the same block as
  the entries. One byte per slot packs 64 per cache line, so a probe usually tests
  occupancy without touching the 16-byte entry at all.

## The overflow-check elision is an interval analysis, and its limits are structural

`src/opt/range.rs` proves an integer operation cannot overflow and drops its check.
[SPEC.md](SPEC.md) 7.2 requires overflow to be an error *when it happens*; it does not
require emitting a check where overflow is impossible, and
[ARCHITECTURE.md](ARCHITECTURE.md) section 3 grants the same reasoning for bounds checks.
D009 is untouched.

Four things it took to make this work, each of which was a bug first:

1. **Globals must be tracked, not just locals.** Every module-scope variable is a global,
   so in `main` — where most loops live — tracking only locals proves nothing. This is
   sound because [SPEC.md](SPEC.md) 9.4 forbids a function from writing a global, so
   within any function only that function's own stores can change one.
2. **The update must be monotone.** `entry[block] = incoming` oscillated forever: widening
   made a header wide, the guard then narrowed the body, and the back edge carried a
   *smaller* range back. It must be `entry[block].join(incoming)`.
3. **Widen only at loop headers.** Widening every block destroyed the guard-narrowed
   bound inside the loop body, which is the whole point of the analysis.
4. **States must be canonical.** A missing key means `Unknown`, so `{}` and
   `{r0: Unknown}` are the same state but compare unequal. Every state is pruned of
   `Unknown` entries or the fixpoint never terminates.

A `BUDGET` caps the worklist and returns "elide nothing" when exhausted, so a bug in
convergence can only cost optimization, never correctness.

**What it cannot do.** It bounds induction variables, because the loop guard narrows them.
It cannot bound a loop *accumulator* — `total = total + i % 7` widens to the full `Int`
range — because that needs the trip count multiplied by the per-iteration increment, which
is scalar evolution, not interval arithmetic. `arith` therefore still carries one check and
still does not vectorize.

Tests in `tests/opt.rs` pin both directions: a bounded induction variable loses its check,
while a parameter, a list element, a parsed value, a loop accumulator and a loose
`n < 1000` guard all keep theirs. The native trap tests remain the backstop.

## Benchmark medians on this machine run 1.3-1.5x the minimum

The development laptop throttles. A single run reported collection work as 30.6 ms when
fifteen runs put the minimum at 24.5 ms, and reported Dovik beating C on `calls` when it
does not. Take the minimum of many runs, and re-measure anything that looks like a new
result before believing it.

## Keep `dovik_release` non-recursive or it stops inlining

Releasing a container releases its elements, so the obvious implementation is recursive —
and LLVM will not inline a recursive function. Every release became a real call, six per
iteration in allocation-heavy code.

The hot path is now a decrement and an early return, with the recursive teardown in a
separate `reclaim` marked `noinline` and `cold`. Both `dovik_retain` and `dovik_release`
carry `always_inline`. That took allocation churn from 7.2x C to 6.0x.

Reintroducing recursion into `dovik_release` silently undoes this. Check with
`objdump -d <binary> | grep -c 'call.*<dovik_release>'`, which should be zero in an
optimized build.

## Measure with at least twenty repetitions

Three repetitions reported `recursion` at 2.75x C when twenty-one put it at 1.53x, and a
single run reported `collections` regressing by 75% when it had not moved. This machine
throttles hard enough that small samples invent results in both directions.

## Hoisting a length out of a loop is safe only when nothing can mutate

`for v in xs` reads `len(xs)` on every iteration, which keeps the loop countable to nobody:
LLVM reports "cannot vectorize uncountable loop" and reloads the length each time.
`src/opt/hoist.rs` moves that read into the preheader when no instruction in the loop body
can mutate a container.

The test for "can mutate" leans on [SPEC.md](SPEC.md) 9.4: a function cannot write a global,
so the only way a call can reach a container is by receiving one as an argument. A call with
only scalar arguments therefore cannot mutate the list being iterated, which is what makes
hoisting provable in real code rather than almost never.

Anything that could mutate — `ListSet`, `DictSet`, `append`, `insert`, `remove_at`,
`remove_key`, or a call taking a reference-typed argument — blocks the hoist for that loop.
Loosening that test without re-deriving the argument means a program that mutates while
iterating can read past the end, because the `for` loop's bounds check is already elided on
the grounds that the cursor is below a *freshly read* length.

## Two benchmarks are bounded by C's vectorization, not by anything fixable

`arith` and `calls` are both scalar in Dovik and SIMD in C — `calls` shows `pmulhuw` and
`paddw` on `xmm` registers, eight 16-bit lanes at once. The overflow check is an early exit
and LLVM will not vectorize a loop with one.

They differ in whether that is closable:

- **`arith` is closable.** Its accumulator grows by `i % 7`, which is `[0, 6]`, over a trip
  count of ten million — so a scalar-evolution analysis could bound it by 6e7 and elide the
  check. Not implemented.
- **`calls` is not.** Its accumulator grows by `value * 3` where `value` comes from a list,
  so no analysis can bound it. The check must stay, so the loop cannot vectorize, so C wins
  this workload for as long as D009 holds.

Removing the overflow checks closes both and is the only thing that does. That trades away
[DECISIONS.md](DECISIONS.md) D009.

## `memory(none)` is unsound for any function that can trap

A function marked `memory(none)` is one LLVM may delete when its result is unused, hoist
out of a loop, or merge with an identical call. A Dovik function containing checked
arithmetic can call `dovik_trap`, which writes to stderr and exits — so marking it pure
lets LLVM **delete an overflow trap**.

`src/backend/purity.rs` therefore requires that a function contain no instruction whose
`runtime_error()` is `Some` before it is considered pure, as well as no memory access. In
practice that means a function is pure only once every check inside it has been elided, or
if it contains no checkable operation at all.

This was caught before shipping. If purity is ever widened, re-derive this: the attribute
is about observable effects, and terminating the program is the most observable effect
there is.

## An unknown integer is bounded by `Int`, which makes guards useful

`src/opt/range.rs` treats a value it knows nothing about as `[Int::MIN, Int::MAX]` rather
than as a lattice top, because every `Int` is in that range by definition — after a checked
operation, after a load, after a list read. Widening targets the same envelope.

That single change is what lets a guard narrow a parameter. In `fib`, `if n < 2` proves
`n >= 2` on the other branch, so `n - 1` and `n - 2` provably fit and lose their checks:
two of three in that function, where previously none were elided.

## Hoisting a loop-invariant load across a call is a pessimisation

Moving `len` out of a loop header looked free and cost 27% on a call-heavy workload. The
hoisted value has to survive every call in the loop body, in a callee-saved register or a
spill slot, and that costs more than re-reading a field that is already in cache.

`src/opt/hoist.rs` therefore skips any loop whose body contains a call, a built-in or a
print. Measured interleaved, since the first attempt looked like a win on a drifting
machine.

## Vectorization is where D009 is actually paid

The remaining gap to C on `arith`, `calls` and list reductions is one thing: LLVM will not
vectorize a loop containing an early exit, and an overflow check is an early exit. C
vectorizes the same loops at width 2 interleaved 2.

Removing **every** check does not close it — `calls` with no checks at all is still 2.4x C,
because the loop also has to be countable and its induction variable identifiable.

Worse, the check that blocks a reduction loop is the accumulator's, and that is the one
check an interval analysis can never elide: `total = total + v` where `v` is read from a
list has no bound. Only `arith` is closable, because its increment is `i % 7`, which is
`[0, 6]`, over a known trip count — that needs scalar evolution.

So reduction loops in Dovik will keep their check, and C will keep winning them, until
either list element ranges are tracked or D009 is traded away. This is the structural cost
of the guarantee, not an implementation deficiency.

## Module-scope variables are globals, which LLVM cannot keep in registers

Every top-level variable lowers to an LLVM global. A loop that accumulates into one
reloads and restores it every iteration, and LLVM will not promote a global across a call
it cannot see through. C's equivalent is a stack local it promotes to a register
immediately.

`src/opt/demote.rs` turns a global into a local of the entry function when no other
function references it. That is sound because [SPEC.md](SPEC.md) 9.4 lets a function *read*
a global but never write one, so a global nothing else mentions is only ever touched by the
entry function and has no observers.

Worth 21% on the call-heavy workload, taking it from 2.77x C to 2.02x — the largest single
improvement to that gap.

The pass rewrites the instructions and leaves the now-unused global in the table rather than
renumbering. LLVM removes it, and the entry function's end-of-program release of a global
that was never written releases zero, which is a no-op.

## Removing the overflow checks makes Dovik slower, and was measured before trusting

The obvious way to close the remaining gap to C is to stop checking. Measured across three
interleaved trials, the unchecked build of the integer workload is 6-28% **slower** than
the checked one, and produces identical output.

The reason is visible in the generated code. Checked: 119 instructions in `main`, no SIMD,
two overflow branches. Unchecked: 212 instructions, still no SIMD, no branches. Without the
overflow intrinsics LLVM unrolls roughly twice as hard and schedules worse, and it still
does not vectorize, because the accumulator is a reduction it cannot identify.

C's advantage on that loop is 50 SIMD instructions out of 61. **Dovik does not get that by
dropping checks** — it would need the reduction recognised, which needs the accumulator
bounded, which needs scalar evolution.

So trading away [DECISIONS.md](DECISIONS.md) D009 buys nothing measurable on this suite.
Anyone proposing it should reproduce the three trials first.

## Scalar evolution is not worth building, and the endpoint was measured

The plan was to bound a loop accumulator so its overflow check could be elided and LLVM
could vectorize the reduction. That plan has a measurable endpoint already available: the
`--omit-runtime-checks-for-benchmarking` build, which removes every check and is therefore
the best any elision analysis could achieve.

Measured interleaved on the integer workload: all checks kept is 1.26x C, all checks
removed is 1.62x C. **Removing them is 29% slower**, because without the overflow
intrinsics LLVM unrolls roughly twice as hard, schedules worse, and still does not
vectorize.

So the highest-risk analysis in the compiler — where a bug is a missing overflow trap —
would have made the benchmark it targeted slower. Do not build it without first re-running
that comparison and finding a different answer.

## Inlining small item storage into a List was tried and reverted

Giving `List` a four-element inline buffer so a small list is one allocation instead of two
measured as noise: +1.7% on allocation churn, -8.7% on string building, nothing outside
±3% elsewhere. The object pool was already avoiding the second allocation in steady state,
so the change bought only locality while growing the object from 48 to 80 bytes.

Reverted. The remaining 5.3x on allocation churn is the object model: a Dovik list carries
a reference count, a length, a capacity and an element tag that C's `malloc(24)` does not.
Closing it means a smaller representation, not a smarter allocator.

## File I/O traps, and `file_exists` is a check rather than a guarantee

[SPEC.md](SPEC.md) 17 forbids a library function from introducing a recoverable error, and
the language has no optional type (U8), so `read_file`, `write_file` and `append_file` trap
on failure. `file_exists` is the predicate that lets a program avoid the trap, the same
shape as `can_parse_int` before `parse_int`.

It is not a guarantee. A file can vanish between the test and the read, and then the read
traps. That race cannot be closed without a result type, and pretending otherwise would be
worse than the trap.

This added a seventh entry to [SPEC.md](SPEC.md) 15.2's runtime error list, `DVK5007`. That
list is normative, so the addition is recorded in the spec and not only in the library doc.

## The entry point now takes argc and argv

The emitted `main` is `define i32 @main(i32 %argc, ptr %argv)` and immediately calls
`dovik_set_arguments`, which stores them for `argument_count` and `argument`. A test
asserting on the old zero-argument signature will fail, which is how this was caught.

`argument(0)` is the first argument the user typed, not the program name. The runtime drops
`argv[0]` when recording the count.

## Text operations are scalar-based, and the runtime walks UTF-8 to prove it

Every index and length in the text functions counts Unicode scalar values, per
[SPEC.md](SPEC.md) 10.4. `substring("héllo wörld", 6, 5)` is `"wörld"`, and
`index_of("héllo wörld", "wörld")` is `6`, not the byte offset `7`.

The runtime searches in bytes, because `memcmp` over UTF-8 cannot produce a false match on
a scalar boundary, and then converts the byte position back to a scalar count. Returning
the byte position would be faster and wrong.

`contains` exists so `index_of` can trap. The error model has no way to return "not found",
so the predicate is the escape, exactly as `file_exists` is for `read_file` and
`can_parse_int` is for `parse_int`. That pattern now appears three times and should be the
default shape for any library function that can fail to find something.

`upper` and `lower` are ASCII only. Full Unicode case conversion can change a string's
scalar count — `ß` uppercases to two scalars — which would make them the only text
functions whose output length is unpredictable. Recorded as an open decision rather than
silently half-done.

## `dovik test` needed no language change, and each test runs in its own process

A test is a module-level function named `test_*` with no parameters, and it fails by raising
a runtime error — which is what `assert` does. That is the whole mechanism. There is no
`test` keyword, no attribute syntax, and nothing in SPEC.md changed for it.

The compiled test binary takes an index and runs the one test it names, and
`cli::pipeline::test` invokes it once per test. That is what lets a failure be reported
without stopping the run: a trap exits the process, and the process only ever held one test.
Running all tests in a single process would report the first failure and nothing after it.

`backend::emit_tests` emits the dispatching entry point. It reuses the normal module
emission and only replaces the body of `main`, so string literal setup, global release and
the leak report all still happen.

## `read_input` reads to end of input, deliberately

A line-at-a-time reader needs an end-of-input predicate beside it, which would have made a
fifth guard-and-accessor pair. Reading everything and handing it to `split` needs no
predicate and composes with the text operations that already exist.

The cost is that a Dovik filter cannot stream: it holds the whole input in memory. That is
the wrong trade for a large file and the right one for the programs Dovik can currently
write.

## A language change goes in the spec before the compiler

[SPEC.md](SPEC.md) 20 requires that a change to syntax or type rules is recorded in the
specification first, and that the implementation must not diverge silently.
[PROPOSAL-OPTIONAL-TYPES.md](PROPOSAL-OPTIONAL-TYPES.md) is therefore a proposal and not a
branch: resolving U8 touches the lexer, the type checker's first flow-sensitive rule, the
IR representation, the runtime, five library signatures and six documents.

Nothing in the compiler depends on that file. If it is rejected, deleting it costs nothing,
which is the point of writing it before writing code.

## A `for` loop over a list typed its element register `Unknown`

`lower_list_loop` created the register receiving `ListGet` with `Type::Unknown`, which the
backend lowers to `i64`. For `Int`, `String`, `List`, and `Dict` elements that is already
the right width, so every existing test passed. For `List<Float>` it emitted
`store double %rN, ptr %sM` where `%rN` was an `i64`, and clang rejected the module. For
`List<Bool>` it stored an `i64` into an `i1` slot.

The element type was available from the container all along. The register now takes it, and
the native corpus iterates a `List<Float>` and a `List<Bool>` so the IR is checked by a
compiler rather than by inspection.

Two lessons. A register type that happens to share a representation with the right answer
hides the bug until a type arrives that does not — `Unknown` is never a correct register
type, only an undetected one. And the interpreter cannot catch this class of defect at all:
it stores a tagged `Value` and does not care what the register claimed, so only the native
backend fails. Any new element type needs a native test, not an interpreter test.

## `sort` needs its own ordering for NaN, because equality is not an ordering

[SPEC.md](SPEC.md) 13 gives `Float` IEEE 754 equality, where NaN is not equal to itself.
That is not an ordering, and a comparison function built from `<` alone is not transitive
once a NaN is present — which lets a sort produce any permutation, drop elements, or in a
hand-written quicksort run off the end of the array.

`sort` therefore defines a total order rather than reusing equality: every NaN is equal to
every other NaN and greater than every number. `-0.0` and `0.0` compare equal, so their
relative order is unspecified and the two implementations do differ there. Nothing may test
it.

The interpreter uses a stable sort and the runtime uses `qsort`, which is not stable. They
agree on every list whose comparison-equal elements are indistinguishable, which is every
list that does not mix `-0.0` with `0.0`.

## `now` and `clock` must stay impure

Both are `InstKind::Builtin`, and `backend::purity` already treats every builtin except
`to_int` and `to_float` as touching memory, so neither gets `memory(none)` and LLVM cannot
hoist one out of a loop or fold two calls into one. That is load-bearing, not incidental:
a `clock()` that LLVM computed once would make every duration zero.

If a later pass ever starts classifying builtins individually, these two belong with the
I/O functions, not with the arithmetic conversions.

## `list_dir` sorts, and dictionaries deliberately do not

The order a file system reports directory entries in is arbitrary and varies between runs,
so an unsorted `list_dir` would make every program that prints a listing
non-reproducible. Sorting costs one pass.

This looks inconsistent with [SPEC.md](SPEC.md) 12.6, which forbids a program from observing
a dictionary's order, but the two cases differ: a dictionary's internal order is meaningless
and exposing it would leak an implementation detail, while a directory listing's
alphabetical order is the thing a reader actually wants.

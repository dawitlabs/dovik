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

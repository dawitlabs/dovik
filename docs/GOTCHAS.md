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

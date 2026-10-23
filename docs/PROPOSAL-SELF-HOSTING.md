# Proposal: Self-Hosting, and the Gap Measured

**Status:** **Draft, not ratified and not implemented.** [ROADMAP.md](ROADMAP.md) Phase 11 wants
Dovik to compile its own compiler. This file counts what the Rust compiler actually uses against
what Dovik has, and concludes that the gap is one feature wide and that the feature is the one
Dovik has most deliberately refused.

## 1. The count

`compiler/src` is **11,573 lines of Rust**. What it uses:

| Construct | Uses | Dovik has it? |
| --- | --- | --- |
| `struct` declarations | 56 | **no** |
| `enum` declarations | 34 | **no** |
| `impl` blocks | 76 | **no** |
| `HashMap` | 56 | yes — `Dict` |
| `Option<T>` | 104 | yes — `T?` ([SPEC.md](SPEC.md) 23) |
| `Result<T, E>` | 25 | **no** |
| `Box<T>` | 13 | not needed — collections are references ([SPEC.md](SPEC.md) 3.4) |
| closures | ~140 | **no** |
| `trait` declarations | 0 | not needed |

Two rows are the good news. Optionals arrived this year and cover 104 uses. `Dict` covers the
56 `HashMap` uses, and `Box` is unnecessary because Dovik's collections already alias
([DECISIONS.md](DECISIONS.md) D013) — a recursive AST in Dovik would not need explicit boxing.
And **zero traits**, so the single largest piece of Rust's type system is not on the critical
path.

## 2. The gap is `struct` and `enum`, and it is not close

**90 type declarations and 76 `impl` blocks.** A compiler is a program about trees, and a tree
needs a type that contains itself. Dovik's six types — `Int`, `Float`, `Bool`, `String`,
`List<T>`, `Dict<K, V>` — cannot express `Expr` or `TokenKind`, and [SPEC.md](SPEC.md) 16 lists
user-defined types as deliberately absent.

The workaround a language without records reaches for is parallel arrays: a `List<Int>` of tags
beside a `List<Int>` of children. That is how one writes a compiler in a language that cannot
describe its own data, and it is unreadable at 11,000 lines. It also throws away the type
checking that is the reason Dovik is statically typed.

**So Phase 11 is blocked on exactly one decision: do user-defined types exist.** Not on
closures — 140 uses, nearly all `map`/`filter`/`unwrap_or_else`, every one of which rewrites as
a loop. Not on `Result` — 25 uses, and an optional plus a diagnostic list covers the compiler's
actual error handling, which collects errors rather than propagating them. Those are
conveniences. Records are not.

## 3. What user-defined types would cost elsewhere

This is the part that makes the decision bigger than it looks, and it is why this proposal
exists rather than a one-line roadmap note.

**It breaks the no-cycle precondition.** Reference counting satisfies
[SPEC.md](SPEC.md) section 14 only because no Dovik program can build a reference cycle, and
that holds because no type can transitively contain itself
([RUNTIME.md](RUNTIME.md) 11). A recursive `struct` — the first thing a compiler needs — makes
cycles constructible immediately. `no_value_can_be_made_to_reach_itself` in `tests/check.rs`
would fail, which is what that test is for.

So user-defined types bring the cycle collector with them. That is the obligation
[RUNTIME.md](RUNTIME.md) 11 names as standing on "whoever adds such a feature", and it is real
work in the runtime: trial deletion or tracing, plus a measurable cost on every release.

**It touches most of the specification.** Declaration syntax, construction, field access —
which [DECISIONS.md](DECISIONS.md) D023 removed entirely when it settled U4 as "functions only,
no member-access expression" — pattern matching or tag tests for enums, equality,
`to_string`, whether records are values or references (3.4 has exactly two categories), and
whether they may be collection elements. D023 is the awkward one: records without field access
are useless, so admitting them reopens a decision that was Accepted on the grounds that
functions were enough.

## 4. The recommendation

**Do not pursue self-hosting as a goal. Let it be a consequence.**

Self-hosting is not a feature a user wants; it is a signal that the language is expressive
enough to write real programs. Chasing it means adding records *in order to* bootstrap, which
is the wrong reason — [SPEC.md](SPEC.md) section 20 and the project's own principle say a
feature needs a stated requirement, not a precedent.

The right order is the reverse. Write larger Dovik programs. When the thing that stops them is
"I cannot describe my data", that is the stated requirement, and records get designed against
it — with the cycle collector in the same change, because
[RUNTIME.md](RUNTIME.md) 11 requires it. Self-hosting then becomes possible, and whether anyone
does it is a separate question from whether the language is good.

**What is worth doing now instead:** nothing in this direction. The three examples are 40 to 70
lines. The gap between them and an 11,000-line compiler is not going to be closed by a language
change made in the abstract.

## 5. Open decisions

- Whether user-defined types ever exist. Everything else here follows from it.
- If they do: records only, or records and tagged unions. A compiler needs both; 34 of the 90
  declarations are enums.
- If they do: how field access coexists with [DECISIONS.md](DECISIONS.md) D023, which removed
  member access as a concept.
- Which cycle-collection algorithm, which [RUNTIME.md](RUNTIME.md) section 8 has held open
  since before it was needed.

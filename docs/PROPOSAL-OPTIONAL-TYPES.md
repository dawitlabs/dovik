# Proposal: Optional Types (resolves U8)

**Status:** **Ratified and implemented.** The normative text is [SPEC.md](SPEC.md) section 23
and the decision is [DECISIONS.md](DECISIONS.md) D025. This file is the design record and is
**not** authoritative; where it disagrees with the specification, the specification is right.

Two things in it were wrong and are corrected where they appear: it never said how a program
builds a present optional, so `some` had to be added (section 5), and it did not consider that
narrowing applies to a variable, which is why `T?` cannot be a collection element
(section 6).

## 1. Why

Four library functions exist only because absence cannot be expressed:

| Guard | Accessor | Exists because |
| --- | --- | --- |
| `file_exists` | `read_file` | a read can fail |
| `can_parse_int` | `parse_int` | text may not be a number |
| `can_parse_float` | `parse_float` | the same |
| `contains` | `index_of` | a needle may be absent |

Each pair is two calls doing one job, and each has a race or a redundancy the type system
cannot remove. `contains` then `index_of` searches the string twice.
`file_exists` then `read_file` can be wrong between the two calls, which
[STDLIB.md](STDLIB.md) has to document as an unavoidable gap.

[SPEC.md](SPEC.md) 12.4 names the same problem for dictionaries: reading a missing key is a
runtime error, and "a safe-lookup API that represents absence explicitly requires a way to
express a value or nothing".

This is not a convenience feature. It is the missing piece that four parts of the library
are currently working around.

## 2. Syntax

A type may be suffixed with `?`:

```dovk
found: Int? = find("hello", "l")
missing: String? = none
```

`none` is a new keyword and the only literal of an optional type. It has no type of its
own; it takes the type required by its context, exactly as an empty list literal does
(11.4). `x = none` without an annotation is therefore a compile-time error, with the same
message shape as `values = []`.

`T?` is a type, so it appears in the two positions any type appears (3.1): after `:` and
after `->`.

**`T??` is a compile-time error.** Nesting adds no expressiveness — there is only one way to
be absent — and it would make narrowing ambiguous.

**`none` is not a value of `T`.** `Int?` and `Int` are different types, and assigning one to
a variable annotated as the other is an error, consistent with 4.1 admitting no implicit
conversion.

## 3. What you may do with an optional

Exactly three things:

1. **Compare it to `none`** with `==` or `!=`, which produces a `Bool`.
2. **Compare it to another `T?`** with `==` or `!=`, if `T` supports equality (13). Two
   `none`s are equal; a `none` and a present value are not.
3. **Narrow it** to `T`, described below.

Everything else is an error. `Int?` is not a number: `found + 1` does not compile, and
neither does `print(found)`. Ordering (`<`) does not apply, because there is no defensible
place to sort `none`.

## 4. Narrowing

An `if` whose condition is exactly `x != none`, where `x` names a variable of type `T?`,
gives `x` the type `T` inside that block:

```dovk
found: Int? = find(line, "=")

if found != none:
    key = substring(line, 0, found)   # found is Int here
else:
    key = line                        # found is Int? here, and unusable
```

Rules that keep this honest:

- Narrowing applies to a **variable**, not an expression. `if f(x) != none:` narrows
  nothing, because there is no binding to narrow.
- Narrowing does not create a declaration, so it does not interact with the shadowing ban
  (9.3). The variable's declared type is unchanged outside the block.
- A variable narrowed in a block **may not be assigned** inside it. Assigning would have to
  either widen the type mid-block or reject a legal `T?` value, and both are worse than the
  restriction. This mirrors 8.3's rule for the loop variable.
- `x == none` narrows the **else** branch, symmetrically.
- `and` does not chain narrowings in the initial version: `if a != none and b != none:`
  narrows neither. Making it work requires the checker to track narrowing through
  short-circuit evaluation, which is worth doing later and is not needed to replace the
  four pairs above.

This is the smallest narrowing rule that makes optionals usable. It is deliberately not
flow-sensitive beyond a single `if`.

## 5. What the library becomes

Each pair collapses into one function:

| Today | Proposed |
| --- | --- |
| `contains` + `index_of` | `find(text: String, needle: String) -> Int?` |
| `can_parse_int` + `parse_int` | `parse_int(text: String) -> Int?` |
| `can_parse_float` + `parse_float` | `parse_float(text: String) -> Float?` |
| `file_exists` + `read_file` | `read_file(path: String) -> String?` |
| `d[k]`, which traps (12.4) | `get(d: Dict<K, V>, key: K) -> V?` |

**What this table missed:** none of these functions lets a *program* build a present
optional, so a user function declared `-> Int?` could return `none` or something it was
handed and nothing else. `some(value)` was added for that, and is the only constructor.

Eight functions become five, the double search in `contains` plus `index_of` disappears, and
the `file_exists` race stops being expressible.

`file_exists` survives as its own question — "is there a file here" is a reasonable thing to
ask without reading it. `contains`, `can_parse_int` and `can_parse_float` are removed.

`d[k]` keeps trapping. A program that knows the key is present should not be forced to
narrow, and 12.4's error stays for the case where it is wrong.

**This is a breaking change.** [SPEC.md](SPEC.md) 20 permits that before 1.0 and says
resolving a section 21 item is expected to break.

## 6. Representation

An optional is **one** machine word, not a value plus a flag. For `String?`, `List?` and
`Dict?` the word is the reference and `none` is zero, which this document predicted. For
`Int?`, `Float?` and `Bool?` the word points at a small pooled reference-counted cell: a
second machine word would have meant aggregate values in an IR built on single words, which
would have touched every stage of the compiler to save a pointer hop in the one place
optionals appear.

The specification promises the cost model rather than the layout
([SPEC.md](SPEC.md) 23.5): a present optional of a reference type allocates nothing, a
present optional of a scalar may allocate, and a hot loop should narrow once.

`T?` may **not** be a dictionary key, for the reason 12.2 excludes `Float`: it would admit a
key whose equality is defensible but whose ordering and hashing invite mistakes, for no
demonstrated need.

**Nor may it be a `List` element or a `Dict` value,** which this document missed. Narrowing
applies to a variable (section 4), so `if xs[0] != none:` narrows nothing and a `List<Int?>`
would be a container a program can fill and test but never read a value out of.

## 7. What this costs to build

Honestly, in the order it has to happen:

1. **Lexer and parser.** `none` becomes a keyword, which is a breaking addition to 2.7.
   `T?` is one token of suffix in `parse_type`.
2. **Type checker.** A new `Type::Optional`, the three permitted operations, the
   no-nesting rule, `none`'s context-dependent typing through the same path that types `[]`,
   and the narrowing rule with its per-block type environment.
3. **IR and lowering.** A representation decision, construction and test instructions, and
   lowering for the narrowed read.
4. **Runtime and backend.** Optional-aware equality, and the flag beside scalar values.
5. **Library migration.** Five signatures change and three functions are removed; every
   test and example that uses them is rewritten.
6. **Documentation.** SPEC.md gains a section, 12.4 and 17 change, U8 is struck from
   section 21, DECISIONS.md records it, STDLIB.md loses three functions.

The narrowing rule in step 2 is the part that deserves the most care: it is the first
place Dovik's type checker becomes flow-sensitive, and the existing checker has no notion
of a type that differs between two blocks.

## 8. The alternative, stated fairly

Keep the guard-and-accessor pairs. They work, every one of them is tested, and the pattern
is now consistent enough that a reader meeting `can_parse_int` can guess `file_exists`.
The cost is eight functions instead of five, one unavoidable race, one double search, and a
type system that cannot say the thing the library keeps needing to say.

The case for building it is that the workaround has appeared four times in two days of
library work. The case against is that it makes the checker flow-sensitive, and that is a
door which does not close again.

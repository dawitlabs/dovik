# Dovik Standard Library

**Status:** First increment implemented. Every name here is a decision, not yet a
contract — [SPEC.md](SPEC.md) 11.2 and U4 deliberately leave these names unfixed,
and [SPEC.md](SPEC.md) 17 specifies the library separately and incrementally.

Two constraints from [SPEC.md](SPEC.md) 17 bind everything below:

1. A library function obeys the specification's type rules and error model. None
   introduces implicit conversion, dynamic typing, or a recoverable error.
2. Until method-call syntax exists (U4), every operation is a function call.
   `numbers.append(4)` is a syntax error; `append(numbers, 4)` is the form.

Library functions live in the built-in scope ([SPEC.md](SPEC.md) 9.1) and cannot be
redeclared ([SPEC.md](SPEC.md) 6.7).

## 1. Already in the Language

Specified in [SPEC.md](SPEC.md) 6.7, listed here for completeness.

| Signature | Notes |
| --- | --- |
| `print(value) -> Unit` | Accepts `Int`, `Float`, `Bool`, `String` |
| `len(value) -> Int` | Accepts `String` (scalars, 10.4), `List`, `Dict` |
| `to_int(x: Float) -> Int` | Truncates toward zero; traps on NaN, infinity, out of range |
| `to_float(n: Int) -> Float` | Never fails |

## 2. List Growth

[SPEC.md](SPEC.md) 11.2 defers these to the library and names none of them.

| Signature | Behaviour |
| --- | --- |
| `append(xs: List<T>, value: T) -> Unit` | Adds one element at the end |
| `insert(xs: List<T>, index: Int, value: T) -> Unit` | Shifts the rest right. `index` may equal `len(xs)`, which appends. Beyond that is a runtime error |
| `remove_at(xs: List<T>, index: Int) -> T` | Removes and returns that element. Out of range is a runtime error |

`insert` accepting `len(xs)` is a deliberate asymmetry with index assignment, which
[SPEC.md](SPEC.md) 11.3 says does *not* append at `len(xs)`. Writing an element and
growing a list are different operations, so they get different bounds.

All three mutate through a reference ([SPEC.md](SPEC.md) 3.4), so a function that
appends to a `List` parameter grows the caller's list — the same rule as `xs[0] = v`
in 9.6.

## 3. Dictionary Presence and Removal

[SPEC.md](SPEC.md) 12.3 defers both to the library.

| Signature | Behaviour |
| --- | --- |
| `has_key(d: Dict<K, V>, key: K) -> Bool` | Whether the key is present |
| `remove_key(d: Dict<K, V>, key: K) -> Bool` | Removes it; returns whether it was there |

`remove_key` returns a `Bool` rather than the removed value, because returning `V`
would have to fail when the key is absent, and the initial language has no way to
express "a value or nothing" (U8). It does not trap on an absent key: removal is
idempotent, unlike the read in 12.4.

There is deliberately **no `keys` function.** [SPEC.md](SPEC.md) 12.6 says a program
cannot observe the order of a dictionary's entries, and returning a `List<K>` would
hand it exactly that. Until ordering is specified or an unordered iteration form
exists, a `Dict` is reachable only by key.

## 4. Text

[SPEC.md](SPEC.md) 17 names "string formatting beyond `print`" and "string-to-number
parsing" as required before realistic programs can be written.

| Signature | Behaviour |
| --- | --- |
| `to_string(value) -> String` | Accepts `Int`, `Float`, `Bool`, `String`. Formats exactly as `print` does (6.7) |
| `parse_int(text: String) -> Int` | Surrounding whitespace is ignored. A failure is a runtime error |
| `parse_float(text: String) -> Float` | As above. A result that is not finite is a failure |
| `can_parse_int(text: String) -> Bool` | Whether `parse_int` would succeed |
| `can_parse_float(text: String) -> Bool` | Whether `parse_float` would succeed |

`to_string` accepting several argument types is a fixed property of that function,
like `print` and `len`, not user-definable overloading.

Parsing comes in pairs because the error model has no recoverable errors
([SPEC.md](SPEC.md) 15.2) and no optional type (U8). A program that cannot accept a
trap asks first:

```dovk
if can_parse_int(text):
    print(parse_int(text))
else:
    print(0)
```

A failed parse reports the existing `invalid conversion` error (`DVK5004`), the same
one `to_int` raises. [SPEC.md](SPEC.md) 15.2 lists that error as "an invalid `to_int`
conversion"; its wording should widen to cover parsing, or parsing should get an
error of its own. That is an open question.

## 5. Files

[SPEC.md](SPEC.md) 17 names file I/O as library surface, and section 16 does not defer it,
so it is library rather than a language change.

| Signature | Behaviour |
| --- | --- |
| `read_file(path: String) -> String` | The whole file as text. A failure is a runtime error |
| `write_file(path: String, contents: String) -> Unit` | Creates or truncates. A failure is a runtime error |
| `append_file(path: String, contents: String) -> Unit` | Creates if absent, otherwise appends |
| `file_exists(path: String) -> Bool` | Whether the path can be opened for reading |

Failures trap rather than return a status, for the same reason parsing does: the error model
has no recoverable errors ([SPEC.md](SPEC.md) 15.2) and no optional type (U8). `file_exists`
is the predicate that lets a program avoid the trap, exactly as `can_parse_int` does:

```dovk
if file_exists(path):
    print(read_file(path))
else:
    print("missing")
```

`file_exists` is a check, not a guarantee. A file can disappear between the test and the
read, and then the read traps. There is no way to express that race away without a result
type.

Reading a file yields text, not bytes. Invalid UTF-8 is replaced rather than rejected, since
`String` is defined as UTF-8 text (3.1) and there is no byte type to return instead.

**This extends [SPEC.md](SPEC.md) 15.2**, which enumerated six runtime errors. File I/O adds
a seventh, `DVK5007`. That list is normative, so the addition is recorded there too.

## 6. Command-Line Arguments

| Signature | Behaviour |
| --- | --- |
| `argument_count() -> Int` | How many arguments the program was given, excluding its own name |
| `argument(index: Int) -> String` | The argument at that index, zero-based. Out of range is a runtime error |

The program's own name is deliberately not argument zero, because a program that wants its
arguments almost never wants its name, and the alternative makes every loop start at one.

## 7. Text Operations

Scalar-based throughout, per [SPEC.md](SPEC.md) 10.4: every index and length counts Unicode
scalar values, never bytes.

| Signature | Behaviour |
| --- | --- |
| `split(text: String, separator: String) -> List<String>` | Splits on every occurrence. Empty fields are kept |
| `join(parts: List<String>, separator: String) -> String` | The inverse of `split` |
| `substring(text: String, start: Int, count: Int) -> String` | `count` scalars from scalar `start`. Out of range is a runtime error |
| `contains(text: String, needle: String) -> Bool` | Whether `needle` occurs |
| `index_of(text: String, needle: String) -> Int` | Scalar position of the first occurrence. Absent is a runtime error |
| `trim(text: String) -> String` | Removes leading and trailing whitespace |
| `starts_with(text: String, prefix: String) -> Bool` | |
| `ends_with(text: String, suffix: String) -> Bool` | |
| `upper(text: String) -> String` | ASCII only |
| `lower(text: String) -> String` | ASCII only |

`contains` pairs with `index_of` the way `file_exists` pairs with `read_file`: the predicate
exists so a program can avoid the trap, because the error model has no way to return
"not found".

```dovk
if contains(line, "="):
    key = substring(line, 0, index_of(line, "="))
```

Three decisions worth knowing:

- **`split` on an empty separator yields the individual scalars.** It is the natural
  reading, it is useful, and the alternative is a runtime error for something a program
  can perfectly well mean.
- **`split` keeps empty fields.** `split("a,,b", ",")` has three elements and
  `split(",a", ",")` has two. Dropping them would lose information a caller may want, and
  `trim` plus a length test recovers the other behaviour.
- **`upper` and `lower` are ASCII only.** `upper("héllo")` is `"HéLLO"`. Full Unicode case
  conversion is locale-dependent and can change a string's scalar count, which would make
  these the only text functions whose output length is unpredictable.
  [SPEC.md](SPEC.md) 10.4 already assigns case conversion to the library without specifying
  its extent, so this is a starting point rather than an answer.

## 8. Not Yet Provided

Named by [SPEC.md](SPEC.md) 17 as library surface, with no implementation and no
chosen names: time, grapheme segmentation and normalization (10.4), full Unicode case
conversion, sorting, standard input, directory operations, and `replace`.

## 9. Open Decisions

- Every name above. They are chosen, not ratified.
- Whether a failed parse keeps `DVK5004` or gets its own runtime error.
- Whether `remove_key` returning `Bool` survives the arrival of optional types (U8),
  or becomes `remove_key(d, k) -> V?`.
- Whether dictionaries ever gain an iteration form, which 12.6 currently forbids.
- Whether `insert` at `len(xs)` should instead be a separate function, keeping every
  index argument in the language uniform at `0 .. len - 1`.
- Whether reading a file should replace invalid UTF-8 or trap on it.
- Whether `argument` should include the program name at index zero.
- Whether file errors deserve distinct codes — not found, permission denied, is a
  directory — rather than one `DVK5007`.
- Whether `upper` and `lower` should grow to full Unicode, and what that does to the
  guarantee that they preserve scalar count.
- Whether `split` should gain a variant that drops empty fields.
- Whether `index_of` should return a scalar position or a value usable directly with
  `substring` — they are the same today and would diverge if byte offsets were ever
  exposed.

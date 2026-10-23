# Dovik Standard Library

**Status:** First increment implemented. Every name here is a decision, not yet a
contract — [SPEC.md](SPEC.md) 11.2 deliberately leaves these names unfixed,
and [SPEC.md](SPEC.md) 17 specifies the library separately and incrementally.

Two constraints from [SPEC.md](SPEC.md) 17 bind everything below:

1. A library function obeys the specification's type rules and error model. None
   introduces implicit conversion, dynamic typing, or a recoverable error.
2. Every operation is a function call ([SPEC.md](SPEC.md) 6.9).
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
| `get(d: Dict<K, V>, key: K) -> V?` | The value, or `none` when the key is absent |
| `has_key(d: Dict<K, V>, key: K) -> Bool` | Whether the key is present |
| `remove_key(d: Dict<K, V>, key: K) -> Bool` | Removes it; returns whether it was there |

`get` is the safe read [SPEC.md](SPEC.md) 12.4 asks for. `d[key]` still traps on a missing
key, deliberately: a program that knows the key is there should not have to narrow, and
12.4's error stays for the case where it is wrong.

`remove_key` still returns a `Bool` rather than the removed value. It could now return `V?`,
and that is left open below rather than changed on the way past.

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
| `parse_int(text: String) -> Int?` | Surrounding whitespace is ignored. `none` when the text is not an integer |
| `parse_float(text: String) -> Float?` | As above. `none` when the result would not be finite |

`to_string` accepting several argument types is a fixed property of that function,
like `print` and `len`, not user-definable overloading.

Parsing returns an optional ([SPEC.md](SPEC.md) 23) rather than trapping, so one call
replaces the guard-and-accessor pair these functions used to come in:

```dovk
parsed = parse_int(text)

if parsed != none:
    print(parsed)
else:
    print(0)
```

`can_parse_int` and `can_parse_float` are gone. They existed only because the error model
had no way to say "not a number", and the one search they saved is now the only search.

## 5. Files

[SPEC.md](SPEC.md) 17 names file I/O as library surface, and section 16 does not defer it,
so it is library rather than a language change.

| Signature | Behaviour |
| --- | --- |
| `read_file(path: String) -> String?` | The whole file as text, or `none` when it cannot be read |
| `write_file(path: String, contents: String) -> Unit` | Creates or truncates. A failure is a runtime error |
| `append_file(path: String, contents: String) -> Unit` | Creates if absent, otherwise appends |
| `file_exists(path: String) -> Bool` | Whether the path can be opened for reading |

`read_file` returns an optional, so there is nothing to guard:

```dovk
text = read_file(path)

if text != none:
    print(text)
else:
    print("missing")
```

This is where optional types earn their place. The old form asked `file_exists(path)` and
then read, and the file could disappear between the two calls — a race
[STDLIB.md](STDLIB.md) had to document as unavoidable. One call cannot have that race.

`file_exists` survives, because "is there a file here" is a reasonable question to ask
without reading the file. It is no longer a guard for anything.

Writing still traps. A failed write is not an absence, and there is nothing useful to
return instead.

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

An executable built with `dovik build` is given its arguments by whatever started it. Under
`dovik run` they go after `--`, which is where `dovik` stops reading options of its own
([CLI.md](CLI.md) 5).

## 7. Text Operations

Scalar-based throughout, per [SPEC.md](SPEC.md) 10.4: every index and length counts Unicode
scalar values, never bytes.

| Signature | Behaviour |
| --- | --- |
| `split(text: String, separator: String) -> List<String>` | Splits on every occurrence. Empty fields are kept |
| `join(parts: List<String>, separator: String) -> String` | The inverse of `split` |
| `substring(text: String, start: Int, count: Int) -> String` | `count` scalars from scalar `start`. Out of range is a runtime error |
| `find(text: String, needle: String) -> Int?` | Scalar position of the first occurrence, or `none` |
| `trim(text: String) -> String` | Removes leading and trailing whitespace |
| `starts_with(text: String, prefix: String) -> Bool` | |
| `ends_with(text: String, suffix: String) -> Bool` | |
| `upper(text: String) -> String` | Full Unicode case mapping, locale-independent. May change the scalar count |
| `lower(text: String) -> String` | As above |
| `replace(text: String, from: String, to: String) -> String` | Every occurrence. An empty `from` changes nothing |

`find` replaces `contains` and `index_of`, which were two calls doing one job and searched
the string twice:

```dovk
at = find(line, "=")

if at != none:
    key = substring(line, 0, at)
```

Three decisions worth knowing:

- **`split` on an empty separator yields the individual scalars.** It is the natural
  reading, it is useful, and the alternative is a runtime error for something a program
  can perfectly well mean.
- **`split` keeps empty fields.** `split("a,,b", ",")` has three elements and
  `split(",a", ",")` has two. Dropping them would lose information a caller may want, and
  `trim` plus a length test recovers the other behaviour.
- **`upper` and `lower` do full Unicode case mapping.** `upper("héllo")` is `"HÉLLO"` and
  `upper("straße")` is `"STRASSE"` — which is seven scalars from six, so **the scalar count
  can change**. That is the price of being correct, and [SPEC.md](SPEC.md) 10.4 now says no
  library function preserves scalar count.

  The mapping is locale-independent: Turkish `I` does not lowercase to `ı`, because the
  language has no locale and silently picking one would be worse than being predictable.
  This is full case mapping, not case *folding* — `upper` is for display, not for
  caseless comparison, and no caseless comparison is provided.

## 8. Standard Input and Assertions

| Signature | Behaviour |
| --- | --- |
| `read_input() -> String` | All of standard input as text. A failure is a runtime error |
| `assert(condition: Bool) -> Unit` | Does nothing when true; a runtime error when false |
| `some(value: T) -> T?` | A present optional ([SPEC.md](SPEC.md) 23.1) |

`read_input` reads to end of input rather than a line at a time, which needs no end-of-input
predicate and composes with `split`:

```dovk
for line in split(read_input(), "\n"):
    print(upper(line))
```

A line-at-a-time reader would need `has_input` beside it, adding a fifth guard-and-accessor
pair, and [PROPOSAL-OPTIONAL-TYPES.md](PROPOSAL-OPTIONAL-TYPES.md) argues the opposite
direction.

`assert` exists so `dovik test` can work without a language change. It raises `DVK5008`,
which extends [SPEC.md](SPEC.md) 15.2 again.

## 9. Sorting

| Signature | Behaviour |
| --- | --- |
| `sort(xs: List<T>) -> Unit` | Orders the list in place, smallest first. `T` must be `Int`, `Float`, or `String` |

`sort` mutates through a reference ([SPEC.md](SPEC.md) 3.4), so sorting a `List`
parameter sorts the caller's list, exactly as `append` does.

There is deliberately **no comparator argument.** The language has no function values
([SPEC.md](SPEC.md) 5.6 rejects a function used as a value), so a comparator cannot be
passed. Ascending order is therefore the whole of sorting until that changes.

`List<Bool>` is rejected. `Bool` has equality (13) but the specification never orders
`false` before `true`, and `sort` is not the place to invent an ordering.

Three decisions worth knowing:

- **`String` sorts by scalar sequence, not by meaning.** `"Apple"` precedes `"apple"`
  precedes `"pear"`, because that is the order of their code points. This matches
  `<` on `String` (6.4) and is not a locale-aware collation.
- **NaN sorts after every number.** Equality in 13 says NaN is not equal to itself,
  which is not an ordering at all, so `sort` needs its own rule: every NaN is treated as
  equal to every other NaN and greater than every number. Without a rule the sort would
  be free to produce any permutation.
- **`-0.0` and `0.0` compare equal, so their relative order is unspecified.** They are
  the only two distinguishable values `sort` may leave in either order, and the two
  implementations do differ there.

## 10. Time

| Signature | Behaviour |
| --- | --- |
| `now() -> Float` | Seconds since 1970-01-01 00:00:00 UTC, including a fraction |
| `clock() -> Float` | Seconds from an unspecified moment, for measuring durations |

Two functions because they answer different questions and only one of them is suitable
for timing:

- `now()` is the wall clock. It is what a program prints or stores, and it **can move
  backwards** when the system clock is corrected.
- `clock()` never moves backwards. Its zero point is not specified and carries no
  meaning, so only the difference between two calls is useful.

```dovk
start = clock()
total = work()
print(to_string(clock() - start) + " seconds")
```

Neither function is a constant, so neither may be folded, hoisted out of a loop, or
computed once and reused. See [GOTCHAS.md](GOTCHAS.md).

There is no calendar: no year, month, day, or formatting. Breaking a timestamp into
fields needs either a `Dict<String, Int>` return or a function per field, and neither
has been chosen.

## 11. Directories

| Signature | Behaviour |
| --- | --- |
| `list_dir(path: String) -> List<String>` | The names inside that directory, sorted. A failure is a runtime error |
| `is_dir(path: String) -> Bool` | Whether the path is a directory |
| `create_dir(path: String) -> Unit` | Creates one directory. A failure is a runtime error |
| `remove_dir(path: String) -> Unit` | Removes one empty directory. A failure is a runtime error |
| `remove_file(path: String) -> Unit` | Removes one file. A failure is a runtime error |

`list_dir` returns names, not paths. Joining is the caller's job, which keeps the
function from having to know about separators.

**`list_dir` is sorted.** The order a file system reports entries in is arbitrary and
varies between runs and machines, which would make every program that prints a listing
non-reproducible. Sorting costs one pass and removes the whole problem. This is the
opposite of [SPEC.md](SPEC.md) 12.6, which forbids observing a dictionary's order — there
the order is meaningless, here the alphabetical order is the thing a reader wants.

`.` and `..` are not included. They are present in every directory and no program wants
them.

The four remaining decisions follow the pattern the rest of the library already uses:

- **`create_dir` creates one level.** A missing parent is a runtime error rather than
  something to create silently, because creating three unintended directories is worse
  than being told the parent is absent.
- **`create_dir` on something that already exists traps.** `is_dir` is the guard, the
  way `file_exists` guards `read_file`.
- **`remove_dir` requires the directory to be empty.** Recursive deletion is one typo
  away from catastrophe, and nothing in the library should make it a single call.
- **`remove_file` refuses directories and `remove_dir` refuses files.** The names say
  which is which, so neither should quietly do the other's job.

Every failure here is `DVK5007`, the same code file I/O uses, because the error model has
no way to say which failure occurred ([PROPOSAL-OPTIONAL-TYPES.md](PROPOSAL-OPTIONAL-TYPES.md)).

## 12. Not Yet Provided

Named by [SPEC.md](SPEC.md) 17 as library surface, still with no implementation:

- **Grapheme segmentation and normalization** (10.4). Case conversion was generated from the
  Rust standard library's own tables ([DECISIONS.md](DECISIONS.md) D024), and these cannot be.

  UAX #29 needs `Grapheme_Cluster_Break`. Most of it could be approximated from properties
  that *are* reachable — general category gives the combining marks, the Hangul jamo blocks
  and the regional indicators are fixed ranges — but `Extended_Pictographic` is not derivable
  from any of them, and rule GB11 needs it. Without GB11 an emoji family sequence counts as
  three clusters instead of one: a wrong answer, silently, which is worse than not having the
  function. Normalization needs the decomposition and composition tables, which are not
  reachable at all.

  So this needs either a vendored extract of the UCD committed to this repository or a
  dependency that carries one. That is a decision about what the project carries, and it is
  open.
- **Calendar operations.** `now()` exists; turning a timestamp into a year and a month
  does not.
- **Descending order.** `sort` only ascends, and with no function values there is no
  comparator to pass. A `reverse` function would cover it in one call and has not been
  chosen.
- **Reading standard input a line at a time.** Deliberately absent, not missing:
  `split(read_input(), "\n")` covers it, and a streaming reader would need an
  end-of-input predicate beside it. The cost is that a filter cannot stream, so it holds
  the whole input in memory — the wrong trade for a large file.

## 13. Open Decisions

- Every name above. They are chosen, not ratified.
- Whether `remove_key` should now return `V?` rather than `Bool`.
- Whether dictionaries ever gain an iteration form, which 12.6 currently forbids.
- Whether `insert` at `len(xs)` should instead be a separate function, keeping every
  index argument in the language uniform at `0 .. len - 1`.
- Whether reading a file should replace invalid UTF-8 or report `none`.
- Whether `argument` should include the program name at index zero.
- Whether file errors deserve distinct codes — not found, permission denied, is a
  directory — rather than one `DVK5007`.
- Whether `upper` and `lower` should grow to full Unicode, and what that does to the
  guarantee that they preserve scalar count.
- Whether `split` should gain a variant that drops empty fields.
- Whether `sort` keeps ascending-only order, or the language grows function values and
  `sort` grows a comparator.
- Whether `List<Bool>` should sort with `false` first rather than being rejected.
- Whether `now` and `clock` are the right names, and whether `now` should return an `Int`
  of whole seconds instead, leaving fractions to `clock`.
- Whether `create_dir` should create missing parents, or gain a second function that does.
- Whether `remove_dir` should ever recurse.
- Whether `list_dir` should return names or full paths.
- Whether `find` should return a scalar position or a value usable directly with
  `substring` — they are the same today and would diverge if byte offsets were ever
  exposed.

# Dovik Examples

Build and run:

```
cd compiler && cargo build && cd ..
./compiler/target/debug/dovik build -o wordcount examples/wordcount.dovk
./wordcount some-file.txt
```

## lib/text.dovk

`wordcount.dovk` and `upper.dovk` import from it ([../docs/SPEC.md](../docs/SPEC.md) 22). It declares `lines`,
`flatten`, and `count_words`, and nothing else in it is imported — a module exports every
module-scope declaration, and the import list at each use site is what decides what arrives.

`dovik build examples/wordcount.dovk` finds it by following the import; there is no manifest
and nothing to configure. The path in the import is relative to the file that writes it.

## wordcount.dovk

Counts lines, words and Unicode scalars in a file, which is the small real-world
command-line program [../docs/BENCHMARKS.md](../docs/BENCHMARKS.md) section 4 asks for.
It reproduces `wc -lwm` exactly, including on UTF-8 input:

```
$ ./wordcount sample.txt
lines      4
words      11
scalars    57
$ wc -lwm < sample.txt
 4 11 57
```

It exercises file reading, argument handling, scalar iteration over a string, string
building, and the guarded-read pattern that the error model requires — `file_exists`
before `read_file`, since a failed read traps.

It also shows what the library still lacks. `split` takes one separator string, not a class
of characters, so `flatten` normalizes newlines and tabs to spaces before `count_words`
splits on a space — and a run of spaces still yields empty pieces that `count_words` has to
skip by hand.

## upper.dovk

A Unix filter. Reads standard input, trims and upper-cases each non-empty line:

```
$ printf 'hello world\n  dovik rules  \n' | ./upper
HELLO WORLD
DOVIK RULES
```

It composes with other tools, which is what `read_input` was added for.

## fib.dovk

Fibonacci two ways, and the only example that imports nothing. It shows the optional type
([../docs/SPEC.md](../docs/SPEC.md) 23) carrying a parse failure:

```
$ ./fib abc
fib: not a number: abc
usage: fib <n>        0 <= n <= 92
$ ./fib 10
fib(10) = 55
0 1 1 2 3 5 8 13 21 34 55
  both agree on  55: true
  loop           0.118 microseconds
  recursion      1.152 microseconds
```

`parse_int` returns `Int?`, and `if requested == none:` narrows the **else** branch to `Int`
(23.4) — which is why every remaining check nests inside that branch rather than sitting
beside it as an `else if`. Narrowing applies to one `if` with one arm.

The upper bound is arithmetic, not arbitrary: `fib(93)` exceeds `Int`, and Dovik traps on
overflow (7.2) instead of wrapping, so the program rejects 93 rather than being killed by it.
`clock()` prices the loop against the naive recursion on the same input.

## Running tests

Any function named `test_*` taking no parameters is a test. `assert` fails one.

```
$ dovik test suite.dovk
ok   test_split_and_join
ok   test_replace
FAIL test_this_one_fails

2 passed, 1 failed, 3 total
```

Each test runs in its own process, so one failure does not hide the rest.

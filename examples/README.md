# Dovik Examples

Build and run:

```
cd compiler && cargo build && cd ..
./compiler/target/debug/dovik build -o wordcount examples/wordcount.dovk
./wordcount some-file.txt
```

## lib/text.dovk

Both examples import from it ([../docs/SPEC.md](../docs/SPEC.md) 22). It declares `lines`,
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

It also shows what the library still lacks. Splitting on whitespace has to be written by
hand, because there is no `split`.

## upper.dovk

A Unix filter. Reads standard input, trims and upper-cases each non-empty line:

```
$ printf 'hello world\n  dovik rules  \n' | ./upper
HELLO WORLD
DOVIK RULES
```

It composes with other tools, which is what `read_input` was added for.

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

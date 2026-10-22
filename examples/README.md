# Dovik Examples

Build and run:

```
cd compiler && cargo build && cd ..
./compiler/target/debug/dovik build -o wordcount examples/wordcount.dovk
./wordcount some-file.txt
```

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

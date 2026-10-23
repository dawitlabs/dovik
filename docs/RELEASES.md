# Dovik Releases and Compatibility

**Status:** The policy below is in force from 0.2.0 onward. Nothing has been released yet —
there are no tags, and 0.1.0 is the version the working tree has carried since the compiler
started producing executables.

This is [ROADMAP.md](ROADMAP.md) Phase 8's "establish release and compatibility practices", and
it is deliberately small. A release process that nobody follows is worse than none, and Dovik
has one author and no users outside this repository.

## 1. What a version number means

`MAJOR.MINOR.PATCH`, and while `MAJOR` is `0` the rules are the honest ones for software this
young:

- **A `MINOR` bump may break anything**, including the language. [SPEC.md](SPEC.md) 20 permits
  it, and the questions that remain open are not answerable without breaking something.
- **A `PATCH` bump breaks nothing** — a fix, a diagnostic, a performance change, a document.
- **`1.0.0` is not scheduled**, and no date should be invented for it. The bar is a language
  that can express its own compiler ([ROADMAP.md](ROADMAP.md) Phase 11), because until then the
  design is still learning what it is missing.

The compiler's version lives in `compiler/Cargo.toml` and is what `dovik --version` prints.
There is one version for the language and the implementation together; splitting them would
promise an independence that does not exist.

## 2. What counts as breaking

A change is breaking if a program that compiled and ran correctly before does not afterwards,
**or** if it runs and gives a different answer. In practice:

| Breaking | Not breaking |
| --- | --- |
| Removing or renaming a keyword, built-in, or type | Adding one |
| Changing what a program computes | Making it compute the same thing faster |
| Rejecting source that used to compile | Accepting source that used to be rejected |
| Changing a diagnostic **code** ([SPEC.md](SPEC.md) 15.3) | Changing a diagnostic **message** |
| Changing the meaning of a CLI flag | Adding a flag |
| Changing the JSON diagnostic schema's fields | Adding a field |

Two things are explicitly *not* a compatibility surface, because the specification says so:
the depth at which recursion exhausts the stack and the allocation at which memory runs out
([SPEC.md](SPEC.md) 15.2), and the memory-management algorithm ([SPEC.md](SPEC.md) 14).
`DOVIK_MAX_DEPTH`, `DOVIK_ALLOC_LIMIT`, `DOVIK_CHECK_LEAKS`, `DOVIK_RUNTIME` and
`DOVIK_WASI_SYSROOT` are test and build hooks, not interface; they may change or disappear.

## 3. What a release requires

In order, and none of it optional:

1. `cargo test --release` passes, with **no suite skipping itself**. The WebAssembly and
   sanitizer suites skip when their toolchain is missing, and a release that let them skip
   would be a release that tested less than it claims. CI fails on exactly that.
2. `bench/run.py` has been run on the release commit if any performance claim changed, and
   [BENCHMARKS.md](BENCHMARKS.md) carries the new numbers with their environment
   ([DECISIONS.md](DECISIONS.md) D017).
3. [CHANGELOG.md](../CHANGELOG.md) has an entry, and every breaking change in it is marked.
4. The version in `compiler/Cargo.toml` is bumped, in its own commit.
5. An annotated tag, `v<version>`, on that commit.

A release is a tag. There are no published artifacts, no package registry, and no installer,
because there is nowhere to publish to yet and inventing a distribution channel before a user
asks for one is the kind of work [DECISIONS.md](DECISIONS.md) D029 declines.

## 4. What is supported

One thing, and the documents that say so are authoritative over any informal claim:
[PLATFORMS.md](PLATFORMS.md) for targets, [STDLIB.md](STDLIB.md) for the library. A target is
supported when it builds and passes the suite, and not before.

There is no support window and no backport policy. The latest version is the only supported
one, and saying otherwise would be a promise nobody is in a position to keep.

## 5. Open decisions

- Whether the language and the compiler ever get separate version numbers. They should not
  until someone writes a second implementation.
- Where releases go when there is somewhere to put them.
- Whether a deprecation period exists before 1.0. Today a breaking change is simply breaking,
  which is cheap while the only program that has to keep working is in this repository.

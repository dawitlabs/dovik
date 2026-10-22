# Proposal: Modules and Imports (resolves SPEC section 16's first bullet)

**Status:** Proposal. Not implemented, and deliberately not yet merged into
[SPEC.md](SPEC.md).

[SPEC.md](SPEC.md) section 20 requires that a change to syntax or scope rules is recorded in
the specification *before* the implementation follows. [SPEC.md](SPEC.md) 16 lists six things
a module system must define — import syntax, visibility, initialization order, dependency
resolution, circular imports, and multi-file compilation — and this document answers all six
so they can be argued with. Nothing in the compiler depends on it.

## 1. Why

One file per program is the last structural limit left. Everything else Dovik needs to write
real programs now exists: 41 library functions, file I/O, arguments, text operations, and
`dovik test`.

The limit shows up in three places already in this repository:

- `examples/wordcount.dovk` defines `count_words` and a whitespace-flattening idiom that
  nothing else can reuse, and both examples repeat `split(text, "\n")` because there is
  nowhere to put it once.
- The test suite for a Dovik program must live in the same file as the program, because
  `dovik test` collects `test_` functions from the one file it is given.
  [CLI.md](CLI.md) 6 already records this as an open question "whether tests belong in a
  separate file that the module system would make possible".
- [ROADMAP.md](ROADMAP.md) Phase 11 (self-hosting) is blocked on this before it is blocked
  on anything else. A compiler is not a single file.

This is not a convenience feature. It is the difference between a language for scripts and a
language something can be built in.

## 2. Syntax

```dovk
import "text.dovk" (split_lines, word_count)
import "lib/format.dovk" (pad as pad_left)
```

An import statement names a file and lists exactly the names it brings in. Three parts, each
chosen for a reason:

**`import` is a new keyword.** [SPEC.md](SPEC.md) 2.7 anticipates this precisely — *"Future
versions will need keywords this list does not reserve — `import` is the obvious example.
Adding one is a breaking change"* — so the cost is already acknowledged. Any program using
`import` as an identifier stops compiling.

**The path is a string literal, not a bare token.** `"lib/format.dovk"` needs no lexer
change, and it keeps `/` and `.` out of the grammar, where they already mean division and
nothing. A bare path would make `import lib/format` lex as a division.

**The name list is mandatory and explicit.** There is no `import "text.dovk"` form that
brings in everything, and no wildcard. Two reasons: a reader can see where every name in a
file came from without opening another file, and adding a declaration to a module can never
break a module that imports from it. With a wildcard, adding a function named `format` to a
widely imported module breaks every importer that already has one.

`as` renames on the way in. It exists because two modules may export the same name, and
without renaming that collision is only fixable by editing one of the two modules — which
stops being possible the moment a module comes from somewhere else. It is the one piece of
this proposal that could be cut without losing anything else.

**Import statements must come before every other top-level statement in the file.**
[SPEC.md](SPEC.md) 8.5 says top-level statements execute in source order, so an import in the
middle of a file would imply its initializers run at that point. Requiring them first makes
initialization order a property of a file's header rather than its body.

## 3. What a module exports

**Every module-scope declaration is importable. There is no `export` keyword and no privacy
mechanism.**

This is safe for variables in a way it would not be in most languages, because
[SPEC.md](SPEC.md) 9.4 already forbids a function from assigning to a module-scope variable.
An imported variable is therefore a constant from the importer's point of view, and no module
can be used as a channel for mutable shared state. U5 stays exactly as open as it is today,
and modules do not become a way around it.

**This is the weakest decision in the document, and it is the one to argue with first.** A
module author gets no way to say "this helper is mine", so any name can be imported and then
cannot be renamed without breaking someone. Three things make it the right call *now* and
only now:

1. The import list is already explicit at the use site, so nothing is imported by accident.
2. Adding privacy later is a breaking change whichever form it takes, so there is no
   cheap-now, cheap-later option to protect.
3. Privacy matters when a module's author and its user are different people. There is no
   dependency management in this proposal, so today they are the same person.

What would change it: a package ecosystem. At that point a module author needs to promise a
surface and keep it, and either an `export` marker or a leading-underscore convention has to
arrive. The convention would cost nothing in the lexer — `_helper` is already a legal
identifier under 2.6 — but it makes a name's meaning depend on its spelling, which is the
kind of decoration the rest of the language avoids. [CLI.md](CLI.md) 6 is already unsure
about the one such convention Dovik has, the `test_` prefix.

## 4. Name resolution, and why section 9 does not change

Imported names land **in the importing module's module scope**, beside its own declarations.
The scope chain in [SPEC.md](SPEC.md) 9.1 stays four deep, exactly as it reads today:

1. Built-in scope
2. Module scope — this module's declarations **and its imported names**
3. Function scope
4. Block scope

That one choice is most of why this proposal is small. Three existing rules then cover the
new cases with no amendment:

- An import colliding with a local declaration is a duplicate declaration in one scope,
  which 9.5 already makes a compile-time error.
- An import colliding with a built-in name is a module-scope declaration reusing a built-in
  name, which 9.3 already forbids. `import "x.dovk" (len)` does not compile.
- Two imports of the same name collide under 9.5, and `as` is the fix.

**Imports do not re-export.** If A imports `split` from B, a module importing from A does not
get `split`. Every file names its own dependencies, which keeps the set of names in scope
derivable from that file's header alone.

**There is no qualified access.** No `text.split(line)`. That form needs a member-access
expression, and U4 deliberately has not decided whether Dovik gets one — adding
`module.name` now would pre-empt the answer, because once `.` means access on the left of a
name nobody will accept it meaning something different for `xs.append(v)`. Flat imports with
an explicit list are the only form that leaves U4 genuinely open.

## 5. Initialization order

Each module's top-level statements run **exactly once**, and **a module is fully initialized
before any statement of a module that imports it runs.**

Order is a depth-first walk of the import graph, visiting each file's imports in source
order, running each module when its imports are done. The entry module runs last. This is
deterministic: the same files produce the same order on every machine.

[SPEC.md](SPEC.md) 8.5's existing check — reject a module-scope call that can reach a
function reading a module-scope variable declared later — extends to the graph without
changing its wording, and the guarantee it buys gets stronger: because an imported module is
*finished* before the importer starts, a program can never read an imported variable whose
initializer has not run. Python, which accepts cycles and hands out half-initialized modules,
cannot promise that.

**The hazard worth stating:** a module's top-level statements run when it is imported, not
when it is the entry file. A library module that calls `print` at top level prints every time
something imports it. A module meant to be imported should declare and not act. Dovik does
not forbid this — a top-level statement is a top-level statement — but it is the first thing
to look for when an import does something surprising.

## 6. Circular imports

**An import cycle is a compile-time error, and the compiler names the cycle.**

The reason is section 5. A cycle makes the initialization order unanswerable: if A's
initializer reads a variable from B and B's reads one from A, no order exists in which both
read an initialized value. The alternatives are to pick an order and hand one module a
half-initialized view of the other, or to forbid the cycle. Dovik rejects the cycle, for the
same reason 8.5 rejects reading a global before its initializer runs.

**The cost is real: mutual recursion stops at the file boundary.** D019 and
[SPEC.md](SPEC.md) 5.5 exist to make two functions able to call each other, and under this
rule they must be in the same file to do so. Two workarounds, both honest: put the mutually
recursive functions in one file, which is usually where they belong anyway; or extract what
they share into a third module that both import.

A refinement is possible and deliberately not proposed: permit a cycle when no module in it
reads a cross-module variable at initialization time, since signatures are collected across
the whole graph anyway and only *data* dependencies make the order unanswerable. That rule is
harder to explain than it is to implement, and a rule nobody can state from memory is worse
than a restriction. It is recorded as an open decision instead.

## 7. Dependency resolution and multi-file compilation

**Paths are relative to the importing file.** Not to the working directory, not to a project
root — to the file containing the `import`, so a directory of modules can be moved together
and keeps working.

- The `.dovk` extension is written out. No implicit extension, no search path, no ambiguity
  about what a name resolves to.
- `..` is permitted. Absolute paths are rejected: they are not portable, and a source file
  that only compiles on one machine is not a source file.
- Two paths that reach the same file are **one module**, resolved by canonicalizing before
  comparing. Otherwise `"./a.dovk"` and `"a.dovk"` would run one file's initializers twice.
- There is no network, no registry, and no version resolution. Every module is a file on
  disk, reached from the entry file.

**`dovik build entry.dovk` discovers the whole graph from the entry file.** No manifest. The
compiler reads the entry file, follows its imports transitively, and compiles everything into
one executable — whole-program, which is what the existing pipeline already does and what
D016's no-interpreter rule and the `-flto=full` link step both assume.

A manifest and dependency management stay out of this proposal. [CLI.md](CLI.md) 5 says both
"belong with the module system, in Roadmap Phase 9", and they do belong *after* it: a manifest
describes a project, and until imports work there is no project to describe. Nothing here
blocks adding one.

`dovik test` keeps collecting `test_` functions from the entry file only. That answers
[CLI.md](CLI.md) 6's question in the smaller direction: a test file becomes a file that
imports the module under test and holds nothing but tests, which needs no new rule.

## 8. What this costs to build

Honestly, in the order it has to happen. The first item is the bulk of the work and is worth
reading before deciding anything else:

1. **Every span needs a file.** `Span` is `{ start, end }` with no file identity, and the
   whole pipeline is built on one source string: `backend::emit(program, source)` computes
   line and column by walking that string, `Report { path, source }` prints diagnostics
   against it, and the interpreter's traps carry a bare span. A second file breaks all of it.
   This is a change to a type that appears in every stage of the compiler, and it is the real
   price of modules — not the parsing, not the scope rules.
2. **Lexer.** `import` becomes a keyword, a breaking addition to 2.7.
3. **Parser.** One statement form, plus the rule that imports come first.
4. **A new stage, before name resolution.** Discover the graph from the entry file,
   canonicalize paths, detect cycles, produce a topological order. This is the one genuinely
   new component, and it is small and self-contained.
5. **Name resolution.** Module scope gains imported bindings; 9.5 and 9.3 collision checks
   extend to them; signature pre-collection spans the graph rather than one file.
6. **Type checking.** No rule changes. The call graph that 8.5 and D019 need becomes
   cross-module, and function indices become global rather than per-file.
7. **IR and lowering.** Globals, functions and interned strings are numbered across the
   graph. The entry point becomes the imported modules' top-level statements, in dependency
   order, followed by the entry module's.
8. **Backend.** Positions resolve against the right file's text, which falls out of item 1.
9. **Documentation.** [SPEC.md](SPEC.md) 16 loses its first bullet and gains a section; 2.7,
   8.5, 9.1 and 9.5 are amended; [DECISIONS.md](DECISIONS.md) records it as D021;
   [CLI.md](CLI.md) 5 and 6 change; [ROADMAP.md](ROADMAP.md) Phase 9 stops being blocked.

Items 2 through 8 are a weekend. Item 1 is not, and it is unavoidable: there is no version of
multi-file compilation where a diagnostic does not have to say which file.

## 9. What this deliberately leaves out

Each of these is a separate decision, and none of them is blocked by shipping the above:

- A project manifest, dependency management, versions, and a registry.
- Privacy and visibility control (section 3).
- Qualified access, `module.name` (section 4).
- Wildcard imports (section 2).
- Re-exports (section 4).
- Nested or namespaced modules. A module is a file; there is no `a.b.c`.
- Conditional or runtime imports. The graph is static and known at compile time.
- Incremental compilation. The whole graph is compiled every time, which is correct and slow,
  and [CLI.md](CLI.md) 6 already lists caching as open.
- Standard-library modules. The 41 built-ins stay in the built-in scope (9.1) and are not
  imported from anywhere. Moving any of them behind an import is a separate breaking change.

## 10. The alternative, stated fairly

Keep one file per program. It has real virtues that are easy to undervalue: there is exactly
one place to look, no resolution rules to learn, no initialization order to reason about, no
cycles to diagnose, and the entire compiler can assume one source string — which it currently
does, in every stage. Several of Dovik's guarantees are cheap to state today precisely
because a program is one file.

The case for building it is that one file per program is what blocks tests living beside the
code they test, two examples sharing a function, and the compiler ever being written in
Dovik. The case against is item 1 of section 8: modules are not a feature bolted onto the
side, they are a change to the type every stage of the compiler carries, and the work is
mostly in the places where modules are not the interesting part.

Unlike [PROPOSAL-OPTIONAL-TYPES.md](PROPOSAL-OPTIONAL-TYPES.md), nothing in the library is
currently working around the absence of modules — there is no pattern appearing four times in
two days. The pressure is structural and comes from the roadmap, not from the code. That is a
reason to decide it deliberately rather than soon.

## 11. Open decisions

- Whether `as` survives, or renaming waits until two modules actually collide.
- Whether privacy arrives as an `export` marker, a leading-underscore convention, or stays
  absent (section 3).
- Whether a cycle with no cross-module initialization dependency should be permitted
  (section 6).
- Whether a module may have top-level statements at all, or whether an imported module should
  be restricted to declarations (section 5).
- Whether `dovik test` should collect tests from the whole graph rather than the entry file.
- Whether the entry file should be allowed to be imported by one of its own dependencies,
  which section 6 already rejects as a cycle but which reads differently to a user.
- Whether paths should be relative to the importing file or to a project root once a manifest
  exists.

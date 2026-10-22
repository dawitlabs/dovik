# Dovik Design Decisions

- **Status:** Draft
- **Companion to:** [SPEC.md](SPEC.md), which is authoritative for language semantics.

This document records why Dovik is the way it is. Each entry states the decision, the reasoning, the trade-off accepted, and whether it is settled.

**Status values:**

- **Accepted** — settled; changing it is a breaking change.
- **Provisional** — intended, but may change once implementation experience arrives.
- **Proposed** — written down, not yet ratified. Needs a call before the code depends on it.

Open questions that have no decision yet live in [SPEC.md](SPEC.md) section 21, not here. When one of them is resolved, it becomes an entry here.

---

## D001: Bootstrap Language

**Decision:** Implement the first compiler in Rust.

**Reason:** Strong type safety, good performance, mature tooling, and usable LLVM bindings.

**Trade-off:** A Rust toolchain is required to build Dovik until self-hosting exists, which is the last roadmap phase.

**Status:** Provisional.

## D002: Code Generation Backend

**Decision:** Use LLVM for initial native code generation.

**Reason:** Mature optimization infrastructure and multi-architecture support without writing a code generator.

**Trade-off:** A large external dependency, a build-time version constraint, and a real risk of LLVM behavior leaking into language semantics. [SPEC.md](SPEC.md) section 18 forbids that leak explicitly: Dovik's semantics are never "whatever LLVM does."

**Status:** Provisional.

## D003: Static Typing with Inference

**Decision:** Dovik is statically typed, and the compiler infers types wherever it safely can.

**Reason:** Catch type errors before execution without annotations on every line.

**Trade-off:** Inference has limits. Empty collection literals and self-referential recursive returns need explicit annotations, and the compiler must say so rather than guessing.

**Status:** Accepted.

## D004: Variable Declarations

**Decision:** Declare by assignment — `age = 23` — with optional annotations — `age: Int = 23`.

**Reason:** Keep ordinary code short while allowing explicit types where they aid readability.

**Trade-off:** Assignment serves as both declaration and reassignment, so the two must be told apart by a rule rather than by syntax. That rule is [SPEC.md](SPEC.md) 9.2, and it only works because shadowing is restricted (D011).

**Status:** Accepted.

## D005: Indentation-Based Blocks

**Decision:** Blocks are defined by indentation — exactly four spaces per level, no tabs, no braces, no semicolons. A block header ends with `:`.

**Reason:** Readable source with minimal syntactic overhead.

**Trade-off:** Indentation becomes significant, so the lexer must track it and diagnose it well. Fixing the level at four spaces removes a whole class of formatting argument but gives up configurability.

**Status:** Accepted.

## D006: Automatic Memory Management

**Decision:** Ordinary programs never deallocate manually. The runtime reclaims unreachable memory.

**Reason:** Remove a large class of memory bugs and keep the language approachable.

**Trade-off:** The implementation must balance memory overhead, throughput, and pause behavior, and it must handle reference cycles. The algorithm is deliberately unchosen — see [RUNTIME.md](RUNTIME.md). What the language guarantees is only that correctness never depends on manual deallocation and that behavior never depends on which algorithm is used ([SPEC.md](SPEC.md) section 14).

**Status:** Accepted as a goal. The algorithm is an open runtime decision.

## D007: Source File Extension and CLI Name

**Decision:** Source files use `.dovk`. The CLI is `dovik`.

**Reason:** Short extension, distinct from the project name so the two are never confused.

**Trade-off:** The one-character difference between `dovik` and `.dovk` is a typo risk and a search-engine problem. Accepted as-is.

**Status:** Accepted.

## D008: Case-Insensitive Built-in Type Names

**Decision:** Built-in type names are case-insensitive. `Int`, `int`, and `INT` name one type. Variable names, function names, and keywords stay case-sensitive.

**Reason:** Removes a pointless source of error for a fixed, small set of names, and makes code written by different people and by coding agents agree.

**Trade-off:** Every case variant of every built-in type name is reserved, so `int` and `string` cannot be variable names. Formatted output normalizes to the capitalized spelling. The rule applies only to built-in type names, which is a special case a reader has to learn.

**Status:** Accepted.

## D009: Int Is 64-bit and Overflow Traps

**Decision:** `Int` is a signed 64-bit integer. Overflow in `+`, `-`, `*`, or negation terminates the program with a runtime error; it never wraps.

**Reason:** Silent wrapping produces wrong answers that survive into production. Dovik prioritizes correctness over raw speed.

**Trade-off:** A check per arithmetic operation. On x86-64 that is a flag test and a branch LLVM can usually keep cheap, but it is not free, and it must hold at every optimization level.

**Status:** Accepted.

## D010: Division Always Produces Float

**Decision:** `/` accepts `Int` or `Float` operands and always produces a `Float`. There is no integer-division operator in the initial version. `%` takes `Int` operands only.

**Reason:** `7 / 2` evaluating to `3` surprises beginners more often than it helps experts.

**Trade-off:** `/` is the only operator that accepts mixed numeric operands, which makes it an exception to "no implicit conversion." The exception is confined to one operator and is stated as a property of that operator rather than as a conversion rule.

A second problem — that `7 / 0` returned infinity while integer overflow trapped — has since been resolved by D020.

**Status:** Accepted.

## D011: Shadowing Is Banned, Except Over Globals

**Decision:** A declaration cannot reuse a visible name, with one exception: a function's parameters and locals may reuse a module-scope variable name, hiding that global for the function's body.

**Reason:** Version 0.2.0 banned shadowing outright while also forbidding functions to assign to globals. Those two rules contradicted each other — inside a function, `x = 1` where `x` is a global could be neither a declaration (shadowing) nor a reassignment (global mutation), so it had no meaning at all.

Resolving it in favor of a blanket ban would be worse: every function's parameter names would have to avoid every module-level variable name, so adding a global could break unrelated functions. Within a single function, where confusion is actually likely, the ban stays.

Three things follow, and [SPEC.md](SPEC.md) 9.6 states them normatively:

- Inside the function, the local binding takes precedence for every mention of that name.
- Assigning to a parameter or local rebinds only that binding. It does not reassign the caller's variable, and it does not touch the global it shadows — which a function could not assign to in any case (D012).
- Mutation is unaffected. A shared object reached through a parameter can still be mutated, and that mutation is visible through every other reference to it, exactly as D013 describes. Rebinding is not mutation.

**Trade-off:** One shadowing case exists, so a reader must know that a parameter wins over a global of the same name. In exchange, functions stay independent of module-level naming.

**Status:** Accepted.

## D012: Functions Cannot Assign to Globals

**Decision:** A function may read module-scope variables but not assign to them. Attempting it is a compile-time error, not an implicit local declaration.

**Reason:** Keeps a function's effects confined to its arguments, its return value, and mutation through reference-typed parameters, which makes programs easier to read and to check.

A function that declares a local or parameter with a global's name does not assign to that global either; it shadows it (D011). Either way, no assignment inside a function changes module state.

**Trade-off:** There is no way to express shared mutable module state, except by mutating a collection the function receives as an argument (D013). For some programs that is a real wall, and no general mechanism is specified yet — see [SPEC.md](SPEC.md) U5.

**Status:** Provisional.

## D013: Reference Semantics for Collections

**Decision:** `Int`, `Float`, and `Bool` are value types. `String`, `List`, and `Dict` are reference types: assigning one or passing it to a function binds another name to the same object rather than copying.

**Reason:** Collections are mutable and memory is managed automatically, so copying on every assignment and call would be both surprising and expensive. Immutability makes the distinction unobservable for `String`.

Passing or assigning one of these objects never copies it implicitly, and the two operations a program can perform on a shared object stay distinct:

- **Mutation** through one reference is visible through every reference to that object.
- **Rebinding** a variable rebinds only that variable. Other variables still refer to the original object.

No new mutation syntax was introduced to say this. The only collection mutation in the initial language remains index assignment, which [SPEC.md](SPEC.md) 11.2 and 12.3 already specified; growing and shrinking a collection stays a standard-library function call under D014.

**Trade-off:** Aliasing becomes observable: a function that mutates a `List` parameter mutates the caller's list. There is no copy syntax in the initial version, so avoiding that requires an explicit standard-library copy.

**Status:** Accepted. Version 0.2.0 left this undefined, which made function calls impossible to implement; [SPEC.md](SPEC.md) 3.4 and 9.6 now specify it. Resolves U1.

## D014: No Method-Call Syntax Initially

**Decision:** The initial version has no `value.method(args)` syntax. Collection and string operations are ordinary function calls.

**Reason:** Method syntax requires member-access expressions and a story about where methods live. Neither is needed to ship the core language.

**Trade-off:** `append(numbers, 4)` reads worse than `numbers.append(4)`, and adding method syntax later would invalidate every existing call site. See [SPEC.md](SPEC.md) U4.

**Status:** Provisional.

## D015: Initial Target Is Linux x86-64

**Decision:** The first and only supported target is Linux x86-64, producing native executables.

**Reason:** One target keeps the first compiler small and lets correctness come before breadth.

**Trade-off:** Nothing else works until it is implemented and tested. No document may describe Windows, macOS, WebAssembly, mobile, desktop, or cross-compilation as available.

**Status:** Accepted.

## D016: No Interpreter or VM

**Decision:** Dovik compiles ahead of time to a native executable. There is no interpreter, bytecode VM, or JIT in the initial version.

**Reason:** Native performance and low memory overhead are core goals, and one execution model is far less work than two.

**Trade-off:** No REPL and no immediate-feedback loop. `dovik run` compiles first and then executes.

**Status:** Accepted.

## D017: Performance Claims Require Benchmarks

**Decision:** No performance claim appears in any Dovik document without a reproducible benchmark behind it. In particular, Dovik does not claim to beat C.

**Reason:** Unsupported performance claims are the fastest way for a new language to lose credibility.

**Trade-off:** Marketing copy stays modest until Phase 8 produces numbers. See [BENCHMARKS.md](BENCHMARKS.md).

**Status:** Accepted.

## D018: Keep the Language Small

**Decision:** The initial language has no classes, inheritance, user-defined types, user-definable generics, interfaces, operator overloading, macros, exceptions, async/await, concurrency, or modules. `List` and `Dict` are built-in parameterized types and are not a general generic system.

**Reason:** A small language can be specified completely, implemented correctly, and tested. Each omitted feature was omitted for lack of a requirement, not an objection in principle.

**Trade-off:** Real programs will want several of these. Each one must earn its way in with a stated requirement, and each is a specification change under [SPEC.md](SPEC.md) section 20.

**Status:** Accepted.

## D019: Mutual Recursion via Signature Pre-Collection

**Decision:** Mutual recursion is supported. The compiler collects every module-level function signature before checking any function body, so a function may call a function declared later in the same module and two or more functions may call each other.

Return-type inference never assumes a function's own result. Inside a recursive cycle the compiler infers what it can from `return` statements that do not depend on a still-unknown function in the cycle, then iterates. A function whose return type is still undetermined when no further progress is possible is genuinely circular and requires an explicit `->` annotation; the compiler names it. A cycle with a base case needs no annotation, because the base case gives inference a starting point.

**Reason:** The previous rule — declaration before reference, applied uniformly — made two functions unable to call each other, which rules out ordinary recursive-descent code and most tree walks. Pre-collecting signatures is the standard fix, needs no new syntax, and costs one extra pass over the module's declarations.

**Trade-off:** Name resolution becomes two passes rather than one, and the compiler must identify recursive cycles in the call graph to decide where an annotation is mandatory. It already needs that call graph to reject a module-scope call reaching a function that reads a not-yet-initialized global ([SPEC.md](SPEC.md) 8.5), so the cost is shared.

Module-scope *statements* still require declaration-before-call, so top-level code reads top to bottom. Only function bodies get module-wide visibility.

**Status:** Accepted. Resolves U6. Specified in [SPEC.md](SPEC.md) 5.5, 8.5, and 9.1.

## D020: Division by Zero Is a Runtime Error

**Decision:** A zero divisor is a runtime error for `/` and for `%`, for every combination of operand types. `7 / 0`, `7.0 / 0.0`, and `0 / 0` all terminate the program with a diagnostic. `/` continues to return `Float` for valid operands (D010), and integer overflow remains a runtime error (D009).

**Reason:** Consistency with D009. Dovik traps integer overflow precisely so that arithmetic without a correct answer fails loudly instead of returning a value that propagates. Returning infinity or NaN from division by zero contradicted that for the same class of mistake, and a NaN travelling through a program is harder to trace back to its origin than a stack trace at the division.

**Trade-off:** `/` departs from IEEE 754, which is a real surprise for anyone expecting floating-point division to be total, and it must be documented wherever Dovik is compared to other languages. It also puts a check on the hot path of every division, including `Float` division, where IEEE 754 would have needed none. That cost is measurable and should be measured ([BENCHMARKS.md](BENCHMARKS.md) section 4).

NaN remains reachable from other operations, such as `inf - inf`, so the rules about NaN equality and `Float` dictionary keys still stand.

**Status:** Accepted. Resolves U2. Specified in [SPEC.md](SPEC.md) 6.2, 7.3, 7.4, and 15.2.

## D021: Modules Are Files, Imports Are Flat and Explicit

**Decision:** A module is one source file. `import "path.dovk" (name, other as alias)` brings exactly the listed names into the importing module's own module scope. There is no wildcard, no qualified access, no re-export, and no privacy marker — every module-scope declaration is importable. Imports must precede every other top-level statement in a file. A module is fully initialized before any module that imports it begins, in a deterministic depth-first order, and a cycle in the import graph is a compile-time error. Paths resolve relative to the importing file; the whole program is discovered from the entry file with no manifest.

**Reason:** Three of the choices are load-bearing and the rest follow from them.

Imported names landing in the *existing* module scope rather than a new scope layer means [SPEC.md](SPEC.md) 9.1 is unchanged and three rules already on the books handle every new collision: 9.5 rejects an import that duplicates a local declaration or another import, and 9.3 rejects one that reuses a built-in name. A module system that needed no new scope rules is the cheapest one available.

Flat imports rather than `module.name` avoid claiming the dot. A qualified form needs a member-access expression, and U4 has not decided whether Dovik gets one; once `.` means module access nobody would accept it meaning something different for `xs.append(v)`.

Rejecting cycles rather than half-initializing keeps the guarantee 8.5 already makes. Python accepts cycles and hands out partly-constructed modules; Dovik would rather name the cycle.

An explicit name list with no wildcard means adding a declaration to a module can never break a module that imports from it, and a reader can see where every name came from without opening another file.

**Trade-off:** Mutual recursion stops at the file boundary. D019 exists to let two functions call each other, and under 22.6 they must share a file to do it — the workaround is to put them in one file, which is usually where they belong, or to extract what they share into a third module.

Everything being importable means a module author cannot mark a helper private, so any name can be depended on and then cannot be renamed. This is acceptable only while a module's author and its user are the same person, which is true today because there is no dependency management. A package ecosystem would force either an `export` marker or a naming convention, and both are breaking changes from here.

The cost in the compiler is not the parsing or the scope rules. It is that a diagnostic must name its file, which the single-source-string pipeline could not do.

**Status:** Accepted. Resolves the first bullet of [SPEC.md](SPEC.md) 16. Specified in [SPEC.md](SPEC.md) 22, with amendments to 2.7, 8.5, 9.1, 15.1, and 16. Design record and rejected alternatives in [PROPOSAL-MODULES.md](PROPOSAL-MODULES.md).

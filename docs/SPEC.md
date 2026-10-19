# Dovik Language Specification

**Version:** Draft 0.1  
**Status:** Under design  
**Source extension:** `.dovk`  
**CLI command:** `dovik`

This document defines the proposed syntax and semantics of Dovik. Rules marked as provisional may change before the first stable language release.

## 1. Design Goals

Dovik aims to provide:

- Readable, minimal syntax.
- Static typing with automatic type inference.
- Native compilation.
- Automatic memory management.
- Low memory overhead.
- Predictable behavior and useful compiler errors.
- A path toward cross-platform development.

Simplicity must not come at the expense of type safety or predictable behavior.

## 2. Source Files and Formatting

Dovik source files use the `.dovk` extension.

Example:

`main.dovk`

### 2.1 Indentation

- Indentation defines code blocks.
- Four spaces are the proposed indentation unit.
- Tabs are not permitted for indentation in the initial specification.
- Blank lines do not change block structure.
- Inconsistent indentation must produce a clear diagnostic.

### 2.2 Comments

A `#` begins a comment outside a string literal.

```dovk
# This is a comment.
name = "Dovik" # This is also a comment.
```

### 2.3 Statements

Statements normally end at a newline. Expressions may span multiple lines only where the syntax explicitly permits them.

Braces are not used to delimit ordinary control-flow or function bodies. Semicolons are not required.

## 3. Variables and Assignment

Variables are introduced through assignment.

```dovk
name = "Dovik"
age = 23
height = 1.75
active = true
```

The compiler infers the initial type from the assigned expression.

### 3.1 Static Types

Once a variable's type is established, its type cannot change.

```dovk
age = 23
age = 24       # Valid: same type.
age = "twenty" # Compile-time error.
```

Reassignment is permitted when the new value has the same type.

### 3.2 Scope and Redeclaration

Variable visibility follows lexical scope.

The precise rules for shadowing, redeclaration, constants, and global variables remain to be finalized.

## 4. Built-in Types

The initial language is expected to include:

| Type     | Purpose                    | Example                         |
| -------- | -------------------------- | ------------------------------- |
| `Int`    | Integer numbers            | `42`                            |
| `Float`  | Floating-point numbers     | `3.14`                          |
| `Bool`   | Boolean values             | `true`                          |
| `String` | Text                       | `"hello"`                       |
| `Unit`   | No meaningful return value | A function that returns nothing |

The exact integer width, floating-point representation, overflow behavior, and numeric conversion rules require explicit decisions before implementation.

### 4.1 Type Annotations

Type inference is the default.

Optional annotations are proposed for cases where an explicit type improves clarity or resolves ambiguity.

```dovk
age: Int = 23
name: String = "Dovik"
```

The exact annotation grammar remains provisional.

### 4.2 Static Type Checking

Type errors must be detected during compilation whenever the compiler has sufficient information.

Dovik must not silently fall back to dynamic typing when inference fails.

When a type cannot be inferred unambiguously, the compiler must request sufficient information or report a useful error.

## 5. Functions

Functions are declared with the `function` keyword.

```dovk
function greet(name)
    print("Hello, " + name)
```

A function that produces a value uses an explicit `return`.

```dovk
function double(number)
    return number * 2
```

Functions are called using parentheses.

```dovk
result = double(5)
print(result)
```

### 5.1 Function Rules

- Function bodies use indentation.
- Parameters have statically determined types.
- Return types must be statically determined.
- Functions without a returned value have type `Unit`.
- Invalid argument types must be reported during compilation.
- Recursive calls must eventually be supported.

The method used to infer parameter and return types, including recursive functions and mutually recursive functions, remains provisional.

## 6. Operators and Expressions

The proposed arithmetic operators are:

`+`, `-`, `*`, `/`, `%`

The proposed comparison operators are:

`==`, `!=`, `<`, `>`, `<=`, `>=`

The logical operators are:

`and`, `or`, `not`

Assignment uses `=`.

### 6.1 Arithmetic

Arithmetic operands must have compatible numeric types.

Implicit numeric conversions, if any, must be explicitly specified.

The meaning of division, including whether `/` always produces a floating-point result, remains undecided.

### 6.2 Comparisons

Comparison expressions produce a `Bool`.

### 6.3 Logical Expressions

Logical operators operate on `Bool` values.

Dovik does not use implicit truthiness. Conditions must evaluate to a boolean value.

### 6.4 String Concatenation

The `+` operator may concatenate two strings.

```dovk
first = "Hello, "
second = "Dovik"
message = first + second
```

Concatenating strings and non-strings requires an explicit conversion or formatting operation.

Operator precedence and evaluation order must be defined before the language is stable.

## 7. Conditions

Dovik uses indentation-based conditional blocks.

```dovk
if age >= 18
    print("Adult")
else if age >= 13
    print("Teenager")
else
    print("Child")
```

Conditions must have type `Bool`.

The language must specify whether every conditional branch is required to return a value when used inside a value-producing expression. Expression-valued conditionals are not included in this initial proposal.

## 8. Loops

### 8.1 For Loops

```dovk
names = ["Dave", "Sara", "Miki"]

for name in names
    print(name)
```

A `for` loop iterates over a supported iterable value.

### 8.2 While Loops

```dovk
count = 0

while count < 3
    print(count)
    count = count + 1
```

### 8.3 Loop Control

The proposed loop-control statements are:

- `break` exits the nearest enclosing loop.
- `continue` skips to the next iteration.

The exact behavior of nested loops and loop-variable scope must be specified.

## 9. Lists

Lists are written using square brackets.

```dovk
numbers = [1, 2, 3]
names = ["Dave", "Sara"]
```

The initial proposal uses homogeneous lists: all elements share one element type.

The specification must define:

- Indexing behavior.
- Out-of-bounds handling.
- Mutation and append operations.
- Empty-list type inference.
- List copying and reference semantics.

An empty list may require an explicit type or sufficient type information from its context.

## 10. Dictionaries

Dictionaries are written using key-value pairs.

```dovk
person = {
    "name": "Dave",
    "age": 23
}
```

The initial proposal uses homogeneous key types and homogeneous value types.

The following remain to be specified:

- Key restrictions.
- Missing-key behavior.
- Mutation.
- Iteration order.
- Equality and hashing.
- Memory and reference semantics.

## 11. Strings

Strings use double quotes.

```dovk
message = "Hello, Dovik"
```

The language must define:

- Escape sequences.
- Unicode representation.
- String indexing behavior.
- String immutability.
- Efficient concatenation and formatting.

These details must be consistent across supported targets.

## 12. Errors and Safety

### 12.1 Compile-Time Errors

Examples include:

- Invalid syntax.
- Undefined variables.
- Incompatible types.
- Invalid function calls.
- Invalid return values.
- Ambiguous type inference.

Compiler diagnostics should identify the source location, explain the problem, and suggest a correction where practical.

### 12.2 Runtime Errors

The runtime must define behavior for failures such as invalid indexing, arithmetic errors, and resource exhaustion.

The exact error-handling syntax and recoverability rules remain undecided.

## 13. Memory Management

Dovik aims to manage ordinary object memory automatically.

Typical programs should not require manual deallocation of each object.

The implementation must define:

- Object lifetime.
- Ownership and reference behavior.
- Cyclic references.
- Reclamation strategy.
- Out-of-memory behavior.

The garbage-collection or other reclamation strategy is an implementation decision until the required semantics and performance constraints are settled.

## 14. Modules and Imports

A module system is required for larger programs.

Its syntax, visibility rules, dependency resolution, package format, and initialization behavior have not yet been finalized.

Simple scripts should eventually be executable without requiring a complex project structure.

## 15. Program Entry Point

The proposed scripting model allows top-level statements to execute in source order.

A mandatory `main` function is not required for simple programs.

The compilation and initialization rules for multi-module applications remain to be defined.

## 16. Standard Library

The initial standard library should prioritize:

- Console input and output.
- Strings and formatting.
- Collections.
- Filesystem operations.
- Command-line arguments.
- Basic error reporting.

Networking, concurrency, advanced collections, and platform-specific APIs can be introduced as the core language matures.

## 17. Compilation and Targets

The compiler should eventually support native executable generation and additional targets.

Linux x86-64 is the proposed first target, subject to confirmation during architecture planning.

Support for other operating systems, CPU architectures, WebAssembly, mobile applications, and desktop applications requires separate implementation and validation.

## 18. Compatibility and Versioning

The language specification and compiler must have identifiable versions.

Breaking changes to syntax or semantics must be documented.

A stable compatibility policy will be defined before a stable release.

## 19. Open Design Decisions

The following must be resolved before the relevant implementation begins:

1. Integer width, overflow, and numeric conversions.
2. Division semantics and operator precedence.
3. Parameter and return-type inference.
4. Optional annotation grammar.
5. Scope, shadowing, and redeclaration.
6. Empty collection inference.
7. Collection mutation and reference semantics.
8. String representation and indexing.
9. Memory reclamation and cyclic references.
10. Error handling.
11. Module and package system.
12. Concurrency model.
13. Target-specific runtime requirements.

## 20. Status

This is a draft, not a finalized language contract.

Every unresolved behavior must be decided, explicitly deferred, or excluded from the first implementation before it becomes a dependency of compiler code.

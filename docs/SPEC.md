# Dovik Language Specification

**Version:** 0.2.0  
**Status:** Initial implementation specification

## 1. Language Overview

Dovik is a statically typed, compiled programming language designed to combine readable syntax, automatic type inference, predictable behavior, and efficient native execution.

Its design goals are:

- Simple, readable syntax.
- Automatic type inference without sacrificing static type safety.
- Native executable generation.
- Automatic memory management.
- Low runtime overhead and efficient memory usage.
- Clear compiler diagnostics.
- Cross-platform support over time.
- Reliable tooling for developers and AI coding agents.

Dovik is intended to support command-line tools, backend services, desktop applications, web applications, and eventually mobile applications.

These are design goals. Actual performance and platform support must be demonstrated through implementation and testing.

## 2. Source Files and Syntax

### 2.1 File Extension

Dovik source files use the `.dovk` extension.

Example:

`main.dovk`

### 2.2 Indentation

Dovik uses indentation to define blocks.

- Four spaces represent one indentation level.
- Tabs are not permitted for indentation.
- Statements within the same block must use consistent indentation.
- Braces are not required to define blocks.
- Semicolons are not required after statements.

Example:

```python
if age >= 18:
    print("Adult")
else:
    print("Minor")
```

### 2.3 Comments

A `#` begins a single-line comment.

```python
# This is a comment
age = 23  # This is also a comment
```

### 2.4 Identifiers

Identifiers name variables and functions.

- Identifiers may contain letters, digits, and underscores.
- An identifier cannot begin with a digit.
- Variable and function names are case-sensitive.
- Reserved keywords cannot be used as identifiers.

For example, `age` and `Age` are different identifiers.

## 3. Built-in Types

Dovik initially defines the following types:

| Type     | Meaning                               |
| -------- | ------------------------------------- |
| `Int`    | Signed 64-bit integer                 |
| `Float`  | 64-bit IEEE 754 floating-point number |
| `Bool`   | Boolean value: `true` or `false`      |
| `String` | Text                                  |
| `Unit`   | Absence of a meaningful return value  |

### 3.1 Case-Insensitive Type Names

Built-in type names are case-insensitive.

The following all refer to the same type:

```python
Int
int
INT
```

This rule applies to built-in type names, not variable or function names.

### 3.2 Static Typing

Dovik checks types at compile time.

A successfully type-checked program cannot silently change a value's type or fall back to dynamic typing.

```python
age = 23
age = 24       # Valid
age = "Dawit"  # Compile-time error
```

## 4. Variables and Type Inference

### 4.1 Automatic Inference

Dovik infers a variable's type from its initial assignment.

```python
age = 23
name = "Dawit"
height = 1.75
is_student = true
```

The inferred types are `Int`, `String`, `Float`, and `Bool`, respectively.

Type annotations are optional for variables.

```python
age: Int = 23
name: String = "Dawit"
```

An explicit annotation must agree with the assigned value's type.

### 4.2 Fixed Variable Types

A variable's type is fixed after its declaration.

```python
age = 23
age = 24       # Valid
age = 25.5     # Type error
age = "Dawit"  # Type error
```

Reassignment changes the value, not the variable's type.

Dovik does not silently convert between incompatible types.

### 4.3 Initialization

A variable must have an initial value or an explicit initialization mechanism before it can be read.

Reading an uninitialized variable is a compile-time error.

### 4.4 Type Inference Failure

If the compiler cannot determine a variable's type safely, it reports an error and requests an explicit annotation where appropriate.

The compiler must not silently assign an arbitrary type.

## 5. Functions

### 5.1 Declaration

Functions use the `function` keyword.

Function parameters require explicit type annotations.

```python
function add(a: Int, b: Int):
    return a + b
```

Parameter types define the function's input contract.

### 5.2 Return Type Inference

Return types are inferred automatically when possible.

```python
function add(a: Int, b: Int):
    return a + b
```

The compiler infers that `add` returns `Int`.

Explicit return annotations are also supported:

```python
function add(a: Int, b: Int) -> Int:
    return a + b
```

The inferred or declared return type must be consistent across every return path.

### 5.3 Return Statements

Functions return values using `return`.

```python
function greet(name: String):
    return "Hello, " + name
```

A function that returns no meaningful value has type `Unit`.

Functions returning `Unit` may omit a return statement. Functions returning another type must return a compatible value on every reachable path.

### 5.4 Function Calls

Arguments must match the declared parameter types.

```python
function multiply(a: Int, b: Int):
    return a * b

result = multiply(3, 5)
```

The compiler infers `result` as `Int`.

This call is invalid:

```python
multiply(3, "five")
```

The compiler reports a type error before execution.

### 5.5 Recursion

Functions may call themselves.

When return-type inference cannot determine a recursive function's return type without circular reasoning, an explicit return annotation is required.

### 5.6 Function Names

Function names are case-sensitive.

Two functions cannot have the same name in the same scope unless a future specification explicitly introduces function overloading.

## 6. Operators and Expressions

### 6.1 Arithmetic

Dovik supports:

| Operator | Meaning                          |
| -------- | -------------------------------- |
| `+`      | Addition or string concatenation |
| `-`      | Subtraction                      |
| `*`      | Multiplication                   |
| `/`      | Division                         |
| `%`      | Integer remainder                |

Arithmetic operands must have compatible numeric types.

`String + String` concatenates strings. Other combinations involving a string and a non-string value are type errors unless an explicit conversion is performed.

### 6.2 Division

The `/` operator always produces a `Float`.

```python
result = 7 / 2
# result is 3.5
```

Integer division is not performed implicitly. A separate integer-division operation may be introduced in a later version.

### 6.3 Remainder

The `%` operator accepts two `Int` operands and returns an `Int`.

The right operand must not be zero.

### 6.4 Comparisons

Dovik supports:

- `==`
- `!=`
- `<`
- `>`
- `<=`
- `>=`

Comparison operators return `Bool`.

Ordering comparisons apply to compatible numeric values and strings. Equality and inequality apply to compatible types that define equality.

### 6.5 Logical Operators

Dovik supports:

- `and`
- `or`
- `not`

Logical operators require `Bool` operands and produce `Bool`.

`and` and `or` use short-circuit evaluation.

### 6.6 Operator Precedence

Operators follow conventional precedence:

1. Parentheses and expression grouping.
2. Unary operators, including `not` and unary minus.
3. Multiplication, division, and remainder.
4. Addition and subtraction.
5. Comparisons and equality.
6. `and`.
7. `or`.

Operators at the same precedence level are evaluated left to right unless explicitly specified otherwise.

Parentheses can be used to make evaluation order explicit.

## 7. Numeric Semantics

### 7.1 Integer Range

`Int` is a signed 64-bit integer.

Its range is:

- Minimum: `-9223372036854775808`
- Maximum: `9223372036854775807`

### 7.2 Integer Overflow

Integer arithmetic must not silently wrap around.

If an operation produces a result outside the `Int` range, the program terminates with a defined runtime error.

This includes integer addition, subtraction, multiplication, negation, and the minimum-integer divided by negative one if integer division is introduced.

### 7.3 Floating-Point Values

`Float` follows IEEE 754 binary64 behavior.

It supports ordinary finite numbers, positive and negative infinity, and NaN.

Floating-point arithmetic may lose precision, as defined by IEEE 754.

### 7.4 Division by Zero

Integer remainder by zero produces a runtime error.

Floating-point division follows IEEE 754 behavior, including infinity or NaN where applicable.

### 7.5 Explicit Conversion

Conversions between numeric types must be explicit.

The standard library will provide conversion functions such as `to_int` and `to_float`.

Converting a `Float` to `Int` truncates toward zero. A conversion that cannot produce a valid `Int`, such as converting NaN, infinity, or an out-of-range value, produces a defined conversion error.

## 8. Conditions and Control Flow

### 8.1 Conditional Statements

Dovik supports `if`, `else if`, and `else`.

```python
if score >= 90:
    print("Excellent")
else if score >= 50:
    print("Pass")
else:
    print("Fail")
```

Conditions must evaluate to `Bool`. Other types do not have implicit truthiness.

### 8.2 Loops

Dovik supports `while` loops and `for` loops.

```python
count = 0

while count < 5:
    print(count)
    count = count + 1
```

A `for` loop iterates over a supported collection or iterable.

```python
for number in numbers:
    print(number)
```

### 8.3 Loop Control

- `break` exits the nearest enclosing loop.
- `continue` skips to the next iteration of the nearest enclosing loop.

Using either outside a loop is a compile-time error.

### 8.4 Execution Order

Top-level statements execute in source order.

A function definition makes that function available from its declaration onward. A function must be declared before it is called.

A simple program does not require a `main` function.

## 9. Scope and Name Resolution

### 9.1 Function Scope

Variables declared inside a function are local to that function.

They cannot be accessed outside it.

### 9.2 Block Scope

Variables declared inside conditional and loop blocks are local to those blocks.

They cannot be accessed after the block ends.

### 9.3 Shadowing

A declaration cannot reuse the name of a variable in an enclosing scope.

This rule avoids ambiguity about which variable an identifier refers to.

### 9.4 Global Variables

Top-level variables belong to the module's global scope.

Functions may read accessible global variables.

Modifying global variables from inside functions is not supported in the initial version. A future version may introduce an explicit mechanism for this.

## 10. Strings

### 10.1 String Values

`String` represents text encoded internally using UTF-8.

String literals use double quotation marks.

```python
name = "Dawit"
message = "Hello, world!"
```

### 10.2 Concatenation

Two strings can be concatenated using `+`.

```python
message = "Hello, " + "Dawit"
```

Implicit conversion of numbers or other types to strings is not supported.

### 10.3 Immutability

Strings are immutable.

Operations that modify text produce a new string rather than changing the original string's contents.

### 10.4 Length and Indexing

String indexing is zero-based and operates on Unicode scalar values rather than UTF-8 bytes.

An indexing operation must check its bounds. An out-of-range index produces a runtime error.

String length counts Unicode scalar values, not bytes or user-perceived grapheme clusters.

The standard library will provide explicit operations for byte length and more advanced Unicode text processing.

## 11. Lists

### 11.1 List Types

Lists contain values of one element type.

```python
numbers = [1, 2, 3]
names = ["Dawit", "Abel"]
```

The inferred types are `List<Int>` and `List<String>`.

Mixed element types are rejected.

```python
numbers = [1, "hello"]  # Type error
```

### 11.2 Mutability

Lists are mutable.

Supported operations will include reading, updating, appending, and removing elements.

Mutating a list cannot change its element type.

### 11.3 Indexing

List indexing is zero-based.

An out-of-range index produces a runtime error.

### 11.4 Empty Lists

An empty list has no element type that can be inferred from its contents.

When the type cannot be inferred from the surrounding context, an explicit annotation is required.

```python
numbers: List<Int> = []
```

## 12. Dictionaries

### 12.1 Dictionary Types

Dictionaries store key-value pairs.

```python
ages = {"Dawit": 23, "Abel": 21}
```

The inferred type is `Dict<String, Int>`.

All keys must share one type, and all values must share one type.

### 12.2 Supported Keys

The initial implementation supports `Int`, `Bool`, and `String` as dictionary key types.

Dictionary keys must satisfy the language's equality and hashing rules.

### 12.3 Mutability

Dictionaries are mutable.

Adding or updating a value must preserve the dictionary's key and value types.

### 12.4 Missing Keys

Looking up a missing key produces a defined runtime error in the initial version.

A future standard-library API may provide a safe lookup operation that represents a missing value explicitly.

### 12.5 Empty Dictionaries

When an empty dictionary's types cannot be inferred from context, explicit type annotations are required.

```python
ages: Dict<String, Int> = {}
```

## 13. Equality and Collection Behavior

Primitive values support equality according to their type's defined semantics.

Lists support structural equality: two lists are equal when they contain the same number of equal elements in the same order.

Dictionaries support structural equality: two dictionaries are equal when they contain the same key-value mappings.

Comparing incompatible types is a compile-time error.

`Float` equality follows IEEE 754 behavior. In particular, NaN is not equal to itself.

The implementation must ensure that dictionary hashing is consistent with key equality.

## 14. Memory Management

### 14.1 Automatic Management

Dovik programs do not require manual memory deallocation for ordinary values.

The runtime is responsible for reclaiming memory that is no longer needed.

### 14.2 Implementation Requirements

The memory-management implementation must address:

- Object allocation and reclamation.
- Memory leaks.
- Cyclic references.
- Collection and string storage.
- Runtime overhead.
- Memory usage and reclamation pauses.

The specific memory-management algorithm is an implementation decision and must be evaluated against Dovik's performance and memory goals.

### 14.3 Implementation Independence

The language's observable behavior must not depend on a particular memory-management algorithm.

The compiler and runtime may evolve their implementation without changing the language's specified semantics.

## 15. Errors

### 15.1 Compile-Time Errors

The compiler rejects programs with errors such as:

- Invalid syntax.
- Undefined variables or functions.
- Duplicate declarations in the same scope.
- Incompatible assignments.
- Invalid function arguments.
- Inconsistent return values.
- Invalid operator operands.
- Invalid condition types.
- Unreachable name references outside their scope.
- Type inference failures that cannot be resolved safely.

### 15.2 Runtime Errors

The initial runtime reports errors such as:

- Integer overflow.
- Integer remainder by zero.
- Invalid numeric conversion.
- Out-of-bounds indexing.
- Missing dictionary keys.

Runtime errors must provide useful diagnostic messages and terminate the affected program predictably.

### 15.3 Diagnostics

Compiler diagnostics should include:

- An error code.
- The source file and location.
- A concise explanation.
- A correction suggestion where practical.

Diagnostic output must be suitable for both humans and automated tools.

## 16. Modules and Imports

The initial implementation supports individual source files.

A module and import system will be introduced when the compiler foundation is stable.

The future module system must define:

- Import syntax.
- Name visibility.
- Module initialization.
- Dependency resolution.
- Circular imports.
- Multi-file compilation.

Until those rules are implemented and specified, multi-module projects are outside the initial language implementation.

## 17. Concurrency

The initial language specification does not define a complete concurrency model.

Concurrency primitives, thread safety, asynchronous operations, and memory-model guarantees will be designed separately before implementation.

The runtime must not promise thread safety for mutable collections unless that guarantee is explicitly provided.

## 18. Standard Library

The standard library will provide core operations for:

- Console input and output.
- Strings.
- Lists and dictionaries.
- Numeric conversion.
- File operations.
- Time and dates.
- Networking.
- Concurrency when supported.

The exact API surface will be defined incrementally.

Standard-library operations must respect Dovik's type system and error-handling rules.

## 19. Compilation Model

Dovik uses a compiler written initially in Rust.

The compiler pipeline is:

1. Lexical analysis.
2. Parsing and abstract syntax tree construction.
3. Name resolution.
4. Type checking and type inference.
5. Intermediate representation generation.
6. Optimization.
7. LLVM code generation.
8. Native linking and executable generation.

LLVM is an implementation backend, not the definition of Dovik's language semantics.

The initial compilation target is Linux x86-64.

## 20. Command-Line Interface

The `dovik` CLI will provide:

- `dovik build`: Compile a program into an executable.
- `dovik run`: Compile and execute a program.
- `dovik check`: Check syntax, names, and types.
- `dovik test`: Run automated tests.
- `dovik format`: Format source code.
- `dovik --version`: Display the compiler version.
- `dovik --help`: Display command help.

The CLI must use predictable exit codes and separate compiler diagnostics from program output.

Machine-readable diagnostics are a goal for automated tooling.

## 21. Platform Support

The initial target is Linux x86-64.

Future targets may include:

- Linux on additional architectures.
- Windows.
- macOS.
- WebAssembly.
- Android.
- iOS.

These targets require their own compiler, runtime, standard-library, and platform-integration work. They are not supported until implemented and tested.

## 22. Performance Requirements

Dovik aims for:

- Competitive native execution speed.
- Low memory overhead.
- Fast startup.
- Reasonable compilation time.
- Small executables where practical.

Performance must be measured using reproducible benchmarks.

Comparisons with C, Rust, Python, and other languages must use equivalent workloads and documented configurations.

Dovik must not claim to be universally faster than C without sufficient evidence.

## 23. Compatibility and Versioning

This document defines the initial language contract.

Changes to syntax, type rules, numeric behavior, scope, or other observable semantics must be documented.

Breaking changes should be identified clearly in release notes.

The implementation must not silently diverge from the specification.

## 24. Development Priorities

Implementation proceeds in this order:

1. Lexer and parser.
2. Abstract syntax tree and syntax diagnostics.
3. Name resolution and scope.
4. Static type checking and inference.
5. Core expression evaluation semantics.
6. Intermediate representation and LLVM code generation.
7. Native executable generation.
8. Functions and control flow.
9. Strings and collections.
10. Runtime and automatic memory management.
11. CLI and developer tooling.
12. Benchmarks, optimization, and platform expansion.
13. Modules and eventual self-hosting.

Each feature must have tests before it is considered complete.

## 25. Specification Status

This document establishes the initial language contract for Dovik.

Implementation details such as the memory-management algorithm, exact LLVM integration, object layout, and cross-compilation strategy remain separate architectural decisions.

A feature must not be advertised as supported until it has been implemented and tested.

The specification should be updated whenever implementation experience reveals a necessary change.

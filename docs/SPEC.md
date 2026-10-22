# Dovik Language Specification

- **Version:** 0.4.0
- **Status:** Initial language contract — pre-implementation
- **Authority:** This document is the authoritative source for Dovik's language semantics. Where any other document in this repository disagrees with it, this document wins and the other document is wrong.

Implementation concerns — the LLVM integration, the memory-management algorithm, object layout, CLI flags, benchmark methodology, and platform rollout — are deliberately **not** specified here. See [ARCHITECTURE.md](ARCHITECTURE.md), [RUNTIME.md](RUNTIME.md), [CLI.md](CLI.md), [PLATFORMS.md](PLATFORMS.md), and [BENCHMARKS.md](BENCHMARKS.md).

Nothing in this document is implemented yet. It describes the contract the first compiler must satisfy, not existing behavior.

---

## 1. Language Overview

Dovik is a statically typed, natively compiled language designed around readable syntax, automatic type inference, predictable behavior, and low runtime overhead.

Design goals:

- Simple, readable syntax.
- Automatic type inference without giving up static type safety.
- Native executable generation.
- Automatic memory management.
- Low runtime overhead and efficient memory usage.
- Clear compiler diagnostics for humans and automated tools.

These are goals. Actual performance must be demonstrated by reproducible benchmarks, and platform support must be demonstrated by working builds and tests.

### 1.1 Scope of This Version

This specification covers single-file programs with variables, functions, control flow, strings, lists, and dictionaries.

The following are **not** part of the initial language and are not specified here: classes, inheritance, user-defined types, user-definable generics, interfaces, macros, operator overloading, exceptions, async/await, concurrency, modules, and method-call syntax.

---

## 2. Source Files and Lexical Structure

### 2.1 File Extension

Dovik source files use the `.dovk` extension, for example `main.dovk`.

### 2.2 Encoding

Source files are UTF-8. A leading byte-order mark, if present, is ignored. Lines end with `\n`; a `\r\n` sequence is accepted and treated as a line ending.

### 2.3 Lines and Statements

One statement per line. There are no statement terminators; a line ending terminates a statement.

An expression may span multiple lines only while an opening `(`, `[`, or `{` is unclosed. Inside those brackets, line endings and indentation carry no meaning.

```dovk
numbers = [
    1,
    2,
    3
]
```

There is no line-continuation character and no single-line block form.

### 2.4 Indentation and Blocks

Blocks are defined by indentation. Braces are not used.

- One indentation level is exactly four spaces.
- Tabs are not permitted for indentation. A tab in a line's leading whitespace is a compile-time error.
- A block header ends with a colon `:`. The block is the following run of lines indented exactly one level deeper than the header.
- Every statement in a block has identical indentation. Indentation that is not a multiple of four spaces, or that skips a level, is a compile-time error.
- A block must contain at least one statement. There is no empty-block or no-op statement.

```dovk
if age >= 18:
    print("Adult")
else:
    print("Minor")
```

Blank lines and comment-only lines may appear anywhere and do not affect block structure.

### 2.5 Comments

`#` begins a comment that runs to the end of the line. There are no block comments.

```dovk
# This is a comment
age = 23  # This is also a comment
```

A `#` inside a string literal is ordinary text.

### 2.6 Identifiers

- An identifier contains ASCII letters, ASCII digits, and underscores.
- An identifier cannot begin with a digit.
- Identifiers are case-sensitive: `age` and `Age` are different identifiers.
- Non-ASCII identifiers are not permitted in the initial version.
- Keywords and built-in type names cannot be used as identifiers (see 2.7 and 3.2).

### 2.7 Keywords

The complete keyword list for the initial version:

```
and  break  continue  else  false  for  function  if  in  not  or  return  true  while
```

Keywords are lowercase and case-sensitive. `If` and `FUNCTION` are ordinary identifiers, not keywords. This differs deliberately from built-in type names, which are case-insensitive (3.2).

Future versions will need keywords this list does not reserve — `import` is the obvious example. Adding one is a breaking change and is governed by section 20.

### 2.8 Literals

**Integer literals** are one or more decimal digits: `0`, `23`, `9223372036854775807`. Digit separators, leading zeros beyond a single `0`, and non-decimal bases are not supported in the initial version.

An integer literal must be in the range `0` to `9223372036854775807`. A larger literal is a compile-time error, **except** that a unary minus applied directly to an integer literal is folded at compile time, which makes `-9223372036854775808` valid.

**Float literals** require a digit on both sides of the decimal point: `1.75`, `0.5`, `3.0`. `.5` and `3.` are syntax errors. Exponent notation is not supported in the initial version. A float literal that is not exactly representable is rounded per IEEE 754; one that overflows to infinity is a compile-time error.

**Boolean literals** are `true` and `false`.

**String literals** are defined in section 10.1.

There is no literal syntax for `Unit`.

---

## 3. Built-in Types

The initial version defines these types and no others:

| Type           | Meaning                                          |
| -------------- | ------------------------------------------------ |
| `Int`          | Signed 64-bit integer                            |
| `Float`        | 64-bit IEEE 754 binary64 floating-point number   |
| `Bool`         | `true` or `false`                                |
| `String`       | Immutable UTF-8 text                             |
| `List<T>`      | Mutable ordered sequence of one element type     |
| `Dict<K, V>`   | Mutable key-value mapping                        |
| `Unit`         | Absence of a meaningful return value             |

`List` and `Dict` are built-in parameterized types. They are **not** evidence of a user-facing generic system: a program cannot declare its own parameterized type, and no other type takes parameters.

### 3.1 Type Syntax

A type appears in exactly two positions: after `:` in a variable declaration or parameter, and after `->` in a function return annotation.

```dovk
count: Int = 0
names: List<String> = []
ages: Dict<String, Int> = {}
scores: List<List<Int>> = []
```

Because type parameters appear only in these positions, `<` and `>` in a type are unambiguous — Dovik has no shift operators, so `>>` closing nested parameters introduces no parsing conflict.

### 3.2 Case-Insensitive Type Names

Built-in type names are case-insensitive. `Int`, `int`, and `INT` all name the same type, as do `float`, `BOOL`, `string`, `list`, `dict`, and `unit`.

Consequence: every case variant of every built-in type name is reserved and cannot be used as a variable or function name. `int = 5` and `function string(...)` are compile-time errors.

This rule applies only to type names. Variable names, function names, and keywords remain case-sensitive (2.6, 2.7).

Documentation and formatted output use the capitalized spelling (`Int`, `List<String>`). `dovik format` is expected to normalize type names to that spelling.

### 3.3 Static Typing

Types are checked at compile time. A program that type-checks cannot change a value's type at runtime or fall back to dynamic typing.

```dovk
age = 23
age = 24       # Valid: reassignment
age = "Dawit"  # Compile-time error
```

### 3.4 Value and Reference Semantics

`Int`, `Float`, and `Bool` are value types. Copying one copies the value.

`String`, `List`, and `Dict` are reference types. Assigning one, or passing one to a function, binds a second name to the *same* object; it does not copy.

```dovk
a = [1, 2, 3]
b = a
b[0] = 99
# a[0] is now 99 — a and b name one list
```

Because `String` is immutable (10.3), this distinction is unobservable for strings.

**Rebinding is not mutation.** Assigning a new object to a name rebinds only that name. Every other name still refers to the original object.

```dovk
a = [1, 2, 3]
b = a
b = [9]
# a is still [1, 2, 3] — rebinding b did not disturb a
```

So two rules work together: mutation through any reference is visible through every reference to that object, and rebinding one name is visible through none of them.

The same split applies to arguments. A function that mutates a reference-typed parameter mutates the caller's object; a function that rebinds that parameter changes nothing the caller can see (9.6).

There is no copy syntax in the initial version. The standard library is expected to provide explicit copy functions.

> Settled as D013 in [DECISIONS.md](DECISIONS.md).

---

## 4. Variables

### 4.1 Declaration and Inference

A variable is declared by assigning to a name that is not already visible (see 9.2 for the exact rule). Its type is inferred from the initializer.

```dovk
age = 23          # Int
name = "Dawit"    # String
height = 1.75     # Float
is_student = true # Bool
```

An optional annotation makes the type explicit. The annotation and the initializer's type must agree exactly; an annotation never triggers a conversion.

```dovk
age: Int = 23
height: Float = 2     # Compile-time error: Int initializer, Float annotation
```

### 4.2 Initialization Is Mandatory

Every declaration has an initializer. There is no declaration-without-value form: `age: Int` on its own is a syntax error, and there is no uninitialized or null state for any variable.

As a result, reading an uninitialized variable is unrepresentable rather than merely diagnosed.

### 4.3 Fixed Types

A variable's type is fixed at declaration. Reassignment changes the value only.

```dovk
age = 23
age = 24       # Valid
age = 25.5     # Compile-time error
age = "Dawit"  # Compile-time error
```

Dovik never converts implicitly between types, in either direction. Assigning an `Int` to a `Float` variable is an error; use an explicit conversion (7.5).

### 4.4 Inference Failure

Where the compiler cannot determine a type safely, it reports an error and names the annotation it needs. It never picks an arbitrary type. The two cases in the initial version are empty collection literals (11.4, 12.5) and recursive functions whose return type is self-referential (5.5).

---

## 5. Functions

### 5.1 Declaration

Functions use the `function` keyword. Every parameter requires an explicit type annotation.

```dovk
function add(a: Int, b: Int):
    return a + b
```

A function declaration is a statement at module level only. Nested function declarations are not supported in the initial version.

Parameters are declarations in the function's scope; two parameters cannot share a name.

### 5.2 Return Type Inference

Return types are inferred when the compiler can determine them from the function body.

```dovk
function add(a: Int, b: Int):
    return a + b        # inferred: Int
```

An explicit annotation uses `->`:

```dovk
function add(a: Int, b: Int) -> Int:
    return a + b
```

Every `return` in a function must produce the same type, and that type must match an explicit annotation if one is present. A function with `return` statements of differing types is a compile-time error — Dovik has no union or sum types to unify them with.

### 5.3 Return Statements and `Unit`

```dovk
function greet(name: String):
    return "Hello, " + name
```

A function with no `return` statement, or whose only `return` statements carry no value, has return type `Unit`.

- A function returning `Unit` may use a bare `return` to exit early, or omit `return` entirely.
- A function returning any other type must return a value on every reachable path. A reachable path that falls off the end is a compile-time error.

`Unit` is a return type only. A `Unit` value cannot be stored, passed, compared, or annotated on a variable: `x = print("hi")` is a compile-time error. Calling a `Unit` function is a statement, not an expression.

### 5.4 Calls

A call passes positional arguments whose types must match the parameter types exactly. No implicit conversion is applied, and there are no default or named arguments in the initial version.

```dovk
function multiply(a: Int, b: Int):
    return a * b

result = multiply(3, 5)   # result: Int
multiply(3, "five")       # Compile-time error
multiply(3)               # Compile-time error: wrong argument count
```

Arguments are evaluated left to right before the call.

Reference-typed arguments are not copied (3.4), so a function that mutates a `List` or `Dict` parameter mutates the caller's object.

### 5.5 Recursion

Both direct and mutual recursion are supported.

The compiler collects every module-level function signature — each parameter's type, and the return type where one is annotated — before it checks any function body. A function body may therefore call any function in the module, including one declared further down the file, and two or more functions may call each other.

```dovk
function factorial(n: Int) -> Int:
    if n <= 1:
        return 1
    return n * factorial(n - 1)
```

The annotations below are optional — inference handles this pair — but they document the contract, which is worth doing for mutually recursive functions:

```dovk
function is_even(n: Int) -> Bool:
    if n == 0:
        return true
    return is_odd(n - 1)

function is_odd(n: Int) -> Bool:
    if n == 0:
        return false
    return is_even(n - 1)
```

**Return-type inference never assumes a function's own result.** Within a set of functions that can reach each other through calls — a *recursive cycle* — the compiler infers what it can from `return` statements whose expressions do not depend on a function in the cycle whose type is still unknown, then repeats with what it has learned, until it makes no further progress.

A function whose return type is still undetermined at that point is genuinely circular, and an explicit `->` annotation is required. The compiler names the function that needs one; it does not loop trying to infer it.

```dovk
function ping(n: Int):      # Compile-time error: annotate the return type
    return pong(n)

function pong(n: Int):      # Compile-time error: annotate the return type
    return ping(n)
```

Neither function above gives inference anywhere to start, so neither return type can be determined. In practice that happens only when every `return` in the cycle routes back through the cycle — which also means the functions cannot terminate. The compiler reports the missing annotation, not the non-termination, which it does not attempt to detect.

A cycle with a base case needs no annotation, because the base case is where inference starts:

```dovk
function describe(n: Int):
    if n == 0:
        return "zero"       # inference starts here: String
    return classify(n)

function classify(n: Int):
    return describe(n - 1)  # resolves to String once describe is known
```

A function outside any cycle never needs an annotation on account of recursion, because every function it calls already has a known return type by the time its own body is checked.

Deep recursion can exhaust the call stack. That behavior is unspecified; see U7 in section 21.

### 5.6 Function Names

Function names are case-sensitive.

Functions and variables share one namespace. A function cannot have the name of a module-level variable, and no two functions can share a name — there is no overloading, and none is planned for the initial version.

---

## 6. Expressions and Operators

### 6.1 Arithmetic

| Operator | Meaning                          |
| -------- | -------------------------------- |
| `+`      | Addition, or string concatenation |
| `-`      | Subtraction, or unary negation   |
| `*`      | Multiplication                   |
| `/`      | Division — always yields `Float` |
| `%`      | Remainder — `Int` operands only  |

**Mixed numeric operands are a compile-time error.** For `+`, `-`, `*`, and `%`, both operands must have the same type. `1 + 2.5` is an error; write `to_float(1) + 2.5`. This follows from 4.3: Dovik does not convert implicitly.

`/` is the single exception, and it is a property of the operator rather than an implicit conversion: see 6.2.

`+` concatenates two `String` operands. Any other combination involving a `String` is a compile-time error, including `"count: " + 5`. There is no implicit stringification; use an explicit conversion.

`+` does not concatenate lists and does not merge dictionaries.

### 6.2 Division

`/` accepts `Int` or `Float` operands, in any combination, and always produces a `Float`. `Int` operands are converted to `Float` before the division.

```dovk
result = 7 / 2        # 3.5
ratio = 7.0 / 2       # 3.5
```

There is no integer division operator and no floor-division operator in the initial version.

**A zero divisor is a runtime error** (15.2), for every combination of operand types. `7 / 0`, `7.0 / 0.0`, and `0 / 0` all terminate the program rather than producing infinity or NaN.

This is a deliberate departure from IEEE 754 for the `/` operator, and it is the same policy as integer overflow (7.2): arithmetic that has no correct answer fails loudly instead of returning a value that propagates silently through the rest of the program. `%` behaves the same way (6.3).

Like `%`, this is a runtime check rather than a proof obligation — the compiler is not required to show a divisor is non-zero, though it may reject a literal zero divisor at compile time.

### 6.3 Remainder

`%` requires two `Int` operands and produces an `Int`. The result takes the sign of the left operand, so `-7 % 3` is `-1`.

A zero right operand is a **runtime** error (15.2), not a compile-time error — the compiler is not required to prove a divisor non-zero. A compiler may additionally reject a literal zero divisor at compile time.

`%` on `Float` operands is a compile-time error.

### 6.4 Comparison

`==` `!=` `<` `>` `<=` `>=` all produce `Bool`.

- Both operands must have the same type. Comparing an `Int` with a `Float`, or any two unrelated types, is a compile-time error.
- `<` `>` `<=` `>=` apply to `Int`, `Float`, and `String`. String ordering is lexicographic by Unicode scalar value — it is not locale-aware or collation-aware.
- `==` and `!=` additionally apply to `Bool`, `List`, and `Dict` (see section 13).
- `Unit` cannot be compared.

### 6.5 Logical Operators

`and`, `or`, and `not` require `Bool` operands and produce `Bool`. No other type has implicit truthiness.

`and` and `or` short-circuit: the right operand is not evaluated when the left operand determines the result.

### 6.6 Precedence and Associativity

Highest to lowest:

| Level | Operators                            | Associativity |
| ----- | ------------------------------------ | ------------- |
| 1     | `(…)` grouping, `f(…)` call, `x[…]` index | left     |
| 2     | unary `-`, `not`                     | right         |
| 3     | `*` `/` `%`                          | left          |
| 4     | binary `+` `-`                       | left          |
| 5     | `<` `>` `<=` `>=`                    | left          |
| 6     | `==` `!=`                            | left          |
| 7     | `and`                                | left          |
| 8     | `or`                                 | left          |

Comparison operators do not chain: `a < b < c` parses as `(a < b) < c` and is therefore a type error, because `Bool` has no ordering.

Operands are evaluated left to right, except where `and` and `or` short-circuit.

Parentheses make grouping explicit and are always permitted.

### 6.7 Built-in Names

A small set of names lives in a built-in scope that encloses the module scope. The initial version requires at least:

- `print(value) -> Unit` — write `value` to standard output followed by a newline. Accepts `Int`, `Float`, `Bool`, or `String`.
- `len(value) -> Int` — the length of a `String` (in Unicode scalar values, 10.4), `List`, or `Dict`.
- `to_int(x: Float) -> Int` and `to_float(n: Int) -> Float` — explicit numeric conversion (7.5).

`print` and `len` accept more than one argument type. That is a fixed property of those two built-ins, not user-definable overloading — a program still cannot declare two functions with one name (5.6), and these built-ins cannot be redeclared at all.

`print` formats its argument as follows: an `Int` in decimal with a leading `-` when negative; a `Bool` as `true` or `false`; a `String` as its own scalar values with no quoting or escaping; a `Float` as the shortest decimal that round-trips to the same value, with `inf`, `-inf`, and `nan` for the non-finite cases. `print` performs no conversion that the language itself would reject — `"count: " + 5` remains a compile-time error (6.1).

Redeclaring a built-in name at module scope is a compile-time error (no shadowing, 9.3).

The rest of the built-in and standard-library surface is not specified here; see section 17.

---

## 7. Numeric Semantics

### 7.1 Integer Range

`Int` is a signed 64-bit two's-complement integer.

- Minimum: `-9223372036854775808`
- Maximum: `9223372036854775807`

### 7.2 Integer Overflow

**Integer arithmetic never silently wraps.** When `+`, `-`, `*`, or unary negation produces a result outside the `Int` range, the program terminates with a defined runtime error (15.2).

This applies at every optimization level. A compiler that constant-folds an overflowing expression must reject it at compile time instead.

`%` cannot overflow: `-9223372036854775808 % -1` is `0`.

### 7.3 Floating-Point Values

`Float` is IEEE 754 binary64. It supports finite values, positive and negative infinity, negative zero, and NaN. Arithmetic rounds to nearest-even and may lose precision, per IEEE 754.

Float arithmetic produces a runtime error in exactly one case: division by zero (6.2). Otherwise it never fails — overflow yields infinity, and operations such as `inf - inf` or `inf * 0.0` yield NaN.

NaN therefore remains reachable, and the rules that mention it still apply: NaN is not equal to itself (section 13), and `Float` cannot be a dictionary key (12.2).

### 7.4 Division and Remainder by Zero

A zero right operand is a runtime error (15.2) for both operators, regardless of operand types:

- `/` with a zero divisor, including `0 / 0` and `0.0 / 0.0` (6.2).
- `%` with a zero right operand (6.3).

Neither produces infinity, NaN, nor a wrapped value. This matches the integer-overflow policy (7.2): the three ways arithmetic can fail to have an answer all fail the same way.

### 7.5 Explicit Conversion

Numeric conversions are always explicit.

- `to_float(n: Int) -> Float` is exact for values up to 2^53 and rounds to nearest beyond it. It never fails.
- `to_int(x: Float) -> Int` truncates toward zero. NaN, either infinity, and any value outside the `Int` range produce a runtime error (15.2).

There is no conversion between `String` and the numeric types in the initial language; parsing and formatting belong to the standard library (section 17).

---

## 8. Control Flow

### 8.1 Conditionals

```dovk
if score >= 90:
    print("Excellent")
else if score >= 50:
    print("Pass")
else:
    print("Fail")
```

`else if` is the two keywords `else` and `if`; there is no `elif`. An `if` may have any number of `else if` clauses and at most one `else`, which must come last.

A condition must have type `Bool`. No other type has implicit truthiness, so `if count:` is a compile-time error.

`if` is a statement. There is no conditional expression in the initial version.

### 8.2 `while`

```dovk
count = 0

while count < 5:
    print(count)
    count = count + 1
```

The condition is re-evaluated before each iteration and must have type `Bool`. A `while` loop whose condition never becomes `false` runs forever; the compiler does not attempt to detect that.

### 8.3 `for`

```dovk
names = ["Dawit", "Abel"]

for name in names:
    print(name)
```

A `for` loop iterates:

- a `List<T>`, yielding each element as `T` in order;
- a `String`, yielding each Unicode scalar value as a single-scalar `String` (10.4).

A `Dict` is **not** iterable in the initial version, because iteration order is not specified (12.6). There is no range syntax, so counting loops use `while`.

There is no user-definable iterator protocol, and no way for a program to make its own value iterable.

The loop variable is declared by the `for` statement, scoped to the loop body, and may not be assigned inside the body. Each iteration rebinds it.

Mutating a `List` while iterating over it is a compile-time error if the compiler can prove it, and otherwise unspecified behavior. Programs must not do it.

### 8.4 Loop Control

- `break` exits the nearest enclosing loop.
- `continue` begins the next iteration of the nearest enclosing loop.

Either one outside a loop is a compile-time error. Neither takes a label or a value.

### 8.5 Execution Order and Declaration Order

Top-level statements execute in source order. A simple program needs no `main` function.

Declaration order works differently for variables than for functions:

- A **variable** must be declared before it is referenced, in source order. This holds at module scope and inside every function and block.
- A **function** is visible throughout the whole module, before and after its own declaration, because signatures are collected before any body is checked (5.5). That is what makes mutual recursion work.
- A **module-scope statement** may call a function only after that function's declaration, so top-level code still reads top to bottom.

Because module-scope variables are initialized in source order, one more check is required: the compiler must reject a module-scope call that can reach — directly, or through further calls — a function that reads a module-scope variable declared after that call. This is a compile-time error, which is what makes it impossible to read a global before its initializer has run.

---

## 9. Scope and Name Resolution

### 9.1 Scopes

Scopes nest in this order, outermost first:

1. **Built-in scope** — `print`, `len`, `to_int`, `to_float`, and the rest of the built-in surface.
2. **Module scope** — top-level variables and all function declarations.
3. **Function scope** — one per function; holds its parameters and locals. A function scope's enclosing scope is the module scope.
4. **Block scope** — one per `if`, `else if`, `else`, `while`, or `for` body.

A name declared in a scope is visible from its declaration to the end of that scope, including nested scopes, and is not visible outside it.

Function names are the one exception: a function declared at module scope is visible across the entire module rather than only after its declaration (8.5).

### 9.2 Declaration Versus Reassignment

`name = expression` means one of two things, and the rule is mechanical:

- If `name` is **not** visible at that point, the statement **declares** `name` in the current scope and infers its type (4.1).
- If `name` **is** visible, the statement **reassigns** it. The expression's type must equal the variable's type (4.3), and the variable must be assignable from here (9.4).

Because shadowing is forbidden (9.3), exactly one of these applies — a program never has to be read twice to know which.

```dovk
total = 0

while total < 10:
    total = total + 1   # reassigns the outer total; does not declare a new one
```

```dovk
if ready:
    message = "go"      # declares message in the if-block
print(message)          # Compile-time error: message is not visible here
```

A declaration is also the only way to introduce a name; there is no separate declaration keyword.

### 9.3 Shadowing

A declaration cannot reuse a name that is already visible, with one deliberate exception.

**Forbidden:** a block-scope declaration reusing a name from an enclosing block, a function scope, or the built-in scope; a function-scope declaration or parameter reusing another name in the same function; a module-scope declaration reusing a built-in name.

**Permitted exception:** a function's parameter or local may reuse the name of a **module-scope** variable. That declaration hides the global for the entire body of that function, and the global is then unreachable there.

```dovk
name = "Dovik"

function greet(name: String):     # permitted: parameter hides the global `name`
    print("Hello, " + name)
```

Inside the function, the local binding wins: every mention of that name refers to the parameter or local, not to the global.

The exception exists because the alternative — globals participating in the no-shadowing rule — makes functions non-composable: adding a module-level variable would break unrelated functions whose parameters happen to share its name. Inside a single function, where confusion is actually likely, shadowing stays banned.

What a shadowing declaration does **not** do is described in 9.6.

> Settled as D011 in [DECISIONS.md](DECISIONS.md).

### 9.4 Global Variables and Assignability

Top-level variables live in module scope. Functions may **read** any module-scope variable visible at the point the function is declared.

A function cannot assign to a module-scope variable. Such an assignment is a compile-time error; it is specifically *not* reinterpreted as declaring a new local.

```dovk
counter = 0

function bump():
    counter = counter + 1   # Compile-time error: cannot assign to a global
```

This keeps a function's observable effects limited to its arguments, its return value, and mutation through reference-typed parameters (3.4).

No mechanism for shared mutable module state is specified. This is a real limitation; see U5 in section 21.

### 9.5 Duplicate Declarations

Two declarations of the same name in the same scope are a compile-time error, whether they are two variables, two functions, or one of each (5.6).

### 9.6 Local Bindings Are Local

A parameter or local variable is a binding of its own. Assigning to it rebinds only that binding, inside the scope that declares it.

Three consequences, which together are the whole rule:

1. **Assigning to a parameter does not affect the caller.** The caller's variable keeps whatever it referred to.
2. **Assigning to a local or parameter that shadows a global does not touch the global** (9.3). The global keeps its value, and the function could not have assigned to it anyway (9.4).
3. **Mutation is a separate matter.** When a parameter holds a reference type, mutating the object it refers to *is* visible to the caller and through every other reference to that object (3.4). Rebinding the parameter is not.

```dovk
items = [1, 2, 3]

function rebind(xs: List<Int>):
    xs = [9]              # rebinds the parameter only

function mutate(xs: List<Int>):
    xs[0] = 99            # mutates the caller's list

rebind(items)             # items is still [1, 2, 3]
mutate(items)             # items is now [99, 2, 3]
```

The distinction is exactly the one in 3.4, viewed from inside a function: rebinding a name is invisible to everyone else, and mutating a shared object is visible to everyone holding a reference to it.

---

## 10. Strings

### 10.1 String Literals

A `String` is text stored internally as UTF-8. Literals are delimited by double quotation marks and may not span lines.

```dovk
name = "Dawit"
message = "Hello, world!"
```

Escape sequences, and only these:

| Escape | Meaning                                |
| ------ | -------------------------------------- |
| `\"`   | Double quotation mark                  |
| `\\`   | Backslash                              |
| `\n`   | Line feed, U+000A                      |
| `\r`   | Carriage return, U+000D                |
| `\t`   | Tab, U+0009                            |
| `\0`   | Null, U+0000                           |
| `\u{…}` | The Unicode scalar with that hex value |

`\u{…}` takes one to six hexadecimal digits. A value above `U+10FFFF`, or in the surrogate range `U+D800`–`U+DFFF`, is a compile-time error.

Any other character after a backslash is a compile-time error. Single quotation marks have no special meaning and need no escape. There is no raw-string, multi-line-string, or interpolation syntax in the initial version.

### 10.2 Concatenation

`+` concatenates two strings. Mixing a string with any other type is a compile-time error (6.1).

```dovk
message = "Hello, " + "Dawit"
```

### 10.3 Immutability

Strings are immutable. Every operation that would alter text produces a new string instead, and `s[0] = "x"` is a compile-time error.

### 10.4 Length and Indexing

A `String` is indexed and measured in **Unicode scalar values**, zero-based — not in UTF-8 bytes and not in user-perceived grapheme clusters.

- `s[i]` yields a `String` containing exactly the scalar at position `i`. There is no character type.
- `len(s)` is the number of scalar values.
- An index outside `0 .. len(s) - 1` is a runtime error (15.2). Negative indexing is not supported.

Two consequences worth stating plainly:

1. `len("é")` is `1` even though the UTF-8 encoding is two bytes, and `len` of a flag emoji or a combining sequence is greater than `1` because those are multiple scalars.
2. Scalar indexing over UTF-8 storage is **O(n)**, not O(1). A loop that indexes a string by position is quadratic. Prefer `for` (8.3). See U3 in section 21.

Byte-level access, grapheme segmentation, normalization, and case conversion belong to the standard library (section 17).

---

## 11. Lists

### 11.1 List Types

A `List<T>` holds any number of values of one element type `T`.

```dovk
numbers = [1, 2, 3]            # List<Int>
names = ["Dawit", "Abel"]      # List<String>
grid = [[1, 2], [3, 4]]        # List<List<Int>>
```

Every element of a literal must have the same type.

```dovk
mixed = [1, "hello"]     # Compile-time error: mixed element types
widened = [1, 2.0]       # Compile-time error: Int and Float do not unify
```

### 11.2 Mutation

Lists are mutable, and a list is a reference type (3.4).

Element assignment is part of the language:

```dovk
numbers[0] = 10
```

The assigned value's type must equal `T`; mutation can never change a list's element type.

Growing and shrinking a list — appending, inserting, removing — is done through **functions**, not methods. Dovik has no method-call syntax in the initial version, so `numbers.append(4)` is a syntax error. The specific function names are standard-library surface and are not fixed here; see section 17 and U4 in section 21.

### 11.3 Indexing

List indexing is zero-based. `xs[i]` reads, `xs[i] = v` writes, and both require `i` to have type `Int`.

An index outside `0 .. len(xs) - 1` is a runtime error (15.2). Negative indexing is not supported. Writing to the index at `len(xs)` does not append; it is an out-of-bounds error.

### 11.4 Empty Lists

An empty literal has no element type to infer, so it needs an annotation:

```dovk
numbers: List<Int> = []   # fine: the annotation supplies the element type
values = []               # Compile-time error: cannot infer element type
```

The compiler infers an empty literal's type from a variable annotation or a parameter type. It does not infer it from a later `append`.

---

## 12. Dictionaries

### 12.1 Dictionary Types

A `Dict<K, V>` maps keys of one type to values of one type.

```dovk
ages = {"Dawit": 23, "Abel": 21}   # Dict<String, Int>
```

Every key in a literal must have the same type, and so must every value. A literal with a repeated key keeps the last occurrence.

### 12.2 Key Types

The initial version permits `Int`, `Bool`, and `String` keys.

`Float` keys are rejected at compile time, because NaN is not equal to itself (7.3) and negative zero equals positive zero, which makes float keys unreliable. `List`, `Dict`, and `Unit` keys are also rejected.

The implementation must keep hashing consistent with equality (section 13).

### 12.3 Mutation

Dictionaries are mutable, and a dictionary is a reference type (3.4).

```dovk
ages["Dawit"] = 24      # updates an existing key
ages["Marta"] = 30      # inserts a new key
```

Index assignment inserts when the key is absent and overwrites when it is present. Key and value types are fixed at declaration and cannot change.

Removal and key-presence tests are standard-library functions, not syntax (11.2, section 17).

### 12.4 Lookup and Missing Keys

`d[k]` reads the value for `k`. The key expression's type must equal `K`.

Reading a key that is not present is a **runtime** error (15.2). There is no default value and no implicit insertion on read.

A safe-lookup API that represents absence explicitly requires a way to express "a value or nothing", which the initial type system does not have. See U8 in section 21.

### 12.5 Empty Dictionaries

As with lists, an empty literal needs an annotation:

```dovk
ages: Dict<String, Int> = {}
```

`{}` is always an empty dictionary, never an empty list or a block.

### 12.6 Iteration Order

Dictionary iteration order is **not specified**, which is why a `Dict` is not iterable in the initial version (8.3). A program cannot depend on, or observe, the order of a dictionary's entries.

---

## 13. Equality

Equality is defined per type:

- `Int`, `Bool`, `String`: equal when their values are identical. String equality compares Unicode scalar sequences, with no normalization — `"é"` as one scalar is not equal to `"é"` as `e` plus a combining accent.
- `Float`: IEEE 754 semantics. NaN is not equal to itself, and `-0.0 == 0.0` is `true`.
- `List`: structural. Equal when lengths match and corresponding elements are equal.
- `Dict`: structural. Equal when they hold the same set of keys and equal values for each. Order is irrelevant (12.6).
- `Unit`: not comparable (5.3).

Comparing values of different types is a compile-time error (6.4).

Structural equality on reference types means two distinct objects can be equal. The initial version provides no identity comparison.

Because `List` and `Dict` equality can recurse into `Float` elements, a list containing NaN is not equal to itself. That follows from IEEE 754 and is intentional.

---

## 14. Memory Management

Dovik programs do not free memory manually. The runtime reclaims memory that is no longer reachable.

The language guarantees:

- A program never needs an explicit deallocation to be correct.
- A reachable value is never reclaimed.
- Unreachable values, including those in reference cycles, are eventually reclaimable.
- Observable program behavior does not depend on which automatic memory-management algorithm the runtime uses.

The language deliberately does **not** specify the algorithm, when collection runs, pause behavior, finalizers, weak references, or whether reclamation is guaranteed before program exit. Those are runtime decisions, measured against Dovik's memory and performance goals; see [RUNTIME.md](RUNTIME.md).

A program must not be written to observe the collector, because the collector is not observable.

---

## 15. Errors

### 15.1 Compile-Time Errors

The compiler rejects, at minimum:

- Invalid syntax, including bad indentation, tab indentation, an empty block, and a missing block-header colon.
- A reference to an undeclared name, or to a name not visible at that point.
- A module-scope call to a function declared later in the file (8.5).
- A module-scope call that can reach a function reading a module-scope variable declared after that call (8.5).
- A function in a recursive cycle whose return type is circular and carries no explicit annotation (5.5).
- Duplicate declarations in one scope (9.5).
- A declaration that shadows a visible name, outside the exception in 9.3.
- Assignment to a global from a function (9.4).
- Assignment of a value whose type differs from the variable's type (4.3).
- Wrong argument count or argument types in a call (5.4).
- Returns of inconsistent types, or a non-`Unit` function with a reachable path that returns nothing (5.2, 5.3).
- Operands of the wrong type for an operator, including mixed `Int`/`Float` arithmetic and `String` mixed with a non-`String` (6.1).
- A non-`Bool` condition in `if` or `while` (8.1, 8.2).
- `break` or `continue` outside a loop (8.4).
- Use of a `Unit` value (5.3).
- An out-of-range numeric literal (2.8).
- An invalid escape sequence or scalar value in a string literal (10.1).
- Mixed element types in a collection literal (11.1, 12.1).
- An invalid dictionary key type (12.2).
- A type-inference failure that cannot be resolved safely (4.4).
- Use of a keyword or built-in type name as an identifier (2.6, 2.7, 3.2).

### 15.2 Runtime Errors

The initial runtime terminates the program with a diagnostic on:

- Integer overflow (7.2).
- Division by zero (6.2), including `0 / 0` and `0.0 / 0.0`.
- Remainder by zero (6.3).
- An invalid `to_int` conversion — NaN, infinity, or out of range (7.5).
- A string, list, or dictionary index out of bounds (10.4, 11.3).
- A missing dictionary key (12.4).

Each must report what failed and where, and must terminate predictably — never continue with a corrupted value.

There is no way to catch or recover from a runtime error. Dovik has no exceptions, and the initial version has no error-value type.

Stack exhaustion and out-of-memory are **not** specified; see U7 in section 21.

### 15.3 Diagnostics

A compiler diagnostic includes a stable error code, the source file and location, a concise explanation, and a suggested correction where one is practical.

Diagnostic output must be consumable by both humans and tools. The machine-readable format is a CLI concern; see [CLI.md](CLI.md).

---

## 16. Deferred Language Features

These are named here only to record that they are deliberately absent from the initial language, with no syntax reserved for them.

- **Modules and imports.** One file per program. A module system must define import syntax, visibility, initialization order, dependency resolution, circular imports, and multi-file compilation before it can be specified.
- **Concurrency.** No threads, no async/await, no memory model. The runtime promises no thread safety for mutable collections.
- **User-defined types**, generics, interfaces, and operator overloading.
- **Exceptions**, `try`/`catch`, and error values.
- **Method-call syntax** (U4), **optional/nullable types** (U8), **integer division** (6.2), **conditional expressions** (8.1), **ranges** (8.3), and **nested functions** (5.1).

Adding any of these is a language change governed by section 20.

---

## 17. Standard Library

The standard library is specified separately and incrementally. It will cover console and file I/O, string operations, list and dictionary operations, numeric conversion and formatting, and time.

Two constraints bind it:

1. Standard-library operations obey this specification's type rules and error model. A library function cannot introduce implicit conversion, dynamic typing, or recoverable errors.
2. Until method-call syntax exists (U4), every library operation is a function call.

The surface needed before realistic programs can be written includes string formatting beyond `print` (6.7), string-to-number parsing (7.5), and list growth (11.2).

---

## 18. Compilation and Platform Facts

Two facts here are normative because programs depend on them. Everything else about the compiler and platforms belongs to [ARCHITECTURE.md](ARCHITECTURE.md) and [PLATFORMS.md](PLATFORMS.md).

- Dovik compiles ahead of time to a native executable. There is no interpreter or VM in the initial version, so a program's semantics are fixed at build time.
- The initial compilation target is **Linux x86-64**. No other platform is supported until it has working builds and tests.

The first compiler is written in Rust and uses LLVM for code generation. Both are implementation choices: **LLVM does not define Dovik's semantics**, and a conforming implementation could use another backend. Nothing in this document may be read as "whatever LLVM does."

---

## 19. Conformance

An implementation conforms when it accepts every program this document defines as valid, rejects every program it defines as invalid, and produces the specified observable behavior for valid programs.

Observable behavior means program output, runtime-error occurrence and kind, and termination. It does not include diagnostic wording, compilation time, memory layout, allocation counts, or when memory is reclaimed.

Where this document says "unspecified", a conforming implementation may do anything, and a program that depends on the outcome is not a conforming Dovik program.

---

## 20. Versioning and Compatibility

This document defines the initial language contract.

- Any change to syntax, type rules, numeric behavior, scope rules, or error classification is a specification change and must be recorded here and in [DECISIONS.md](DECISIONS.md).
- Breaking changes must be identified in release notes.
- The implementation must not diverge silently. Where implementation experience shows a rule is wrong, the rule changes here first.
- Resolving anything in section 21 is expected to be a breaking change and is acceptable before version 1.0.

---

## 21. Unresolved Questions

Each of these is open on purpose. Inventing an answer would be worse than naming the trade-off. Every entry says what it blocks, because several roadmap phases depend on them — see [ROADMAP.md](ROADMAP.md).

Identifiers are stable. A resolved question keeps its number and is listed below rather than renumbering the rest, so references from other documents stay valid.

**Resolved.**

| Question | Resolution | Record |
| -------- | ---------- | ------ |
| **U1** — reference semantics for collections | Reference semantics for `String`, `List`, and `Dict`; rebinding is not mutation (3.4, 9.6) | [DECISIONS.md](DECISIONS.md) D013 |
| **U2** — division by zero | A zero divisor is a runtime error for all operand types, including `0 / 0` (6.2, 7.4) | [DECISIONS.md](DECISIONS.md) D020 |
| **U6** — mutual recursion | Supported; signatures are collected before bodies are checked, and only a genuinely circular return type needs an explicit annotation (5.5, 8.5) | [DECISIONS.md](DECISIONS.md) D019 |

The shadowing question that D011 settled was never a numbered entry here; it is recorded in [DECISIONS.md](DECISIONS.md) D011 and specified in 9.3 and 9.6.

**Still open.**

### U3 — String indexing cost and unit

10.4 indexes by Unicode scalar over UTF-8 storage, which is O(n) per index and conflicts with the low-overhead goal.

Alternatives: index by scalar and accept O(n), with `for` as the fast path; index by byte, which is O(1) but lets a program split a scalar; store UTF-32, which is O(1) but up to four times the memory; keep scalar indexing but add a cursor type, which is a new type.

**Blocks:** the string representation in [RUNTIME.md](RUNTIME.md), and Roadmap Phase 6.

### U4 — Method-call syntax versus functions

The initial version has no method syntax, so collection operations are functions: `append(xs, v)` rather than `xs.append(v)`. The library surface in section 17 depends on which way this goes, and switching later invalidates every call site.

Alternatives: functions only; add `value.function(args)` as sugar that resolves to the same function; add real methods, which means a member-access expression and a bigger type system.

**Blocks:** the standard-library API, Roadmap Phases 5 and 6.

### U5 — Shared mutable module state

9.4 forbids assigning to a global from a function, and no replacement mechanism exists. A program that needs a counter shared across functions cannot express one except by threading a `List` through every call.

Alternatives: an explicit `global` declaration inside the function that assigns; mutable-reference parameters; keep the restriction and rely on reference-typed parameters; allow module-level assignment from functions and accept the readability cost.

**Blocks:** realistic Phase 5 programs. Needed before the benchmark suite in Phase 8, which will want shared state.

### U7 — Stack exhaustion and out-of-memory

Neither has specified behavior (15.2). A deeply recursive program's failure mode is currently undefined.

Alternatives: let the OS terminate the process; a guard page plus a diagnostic; a configurable recursion-depth limit. Out-of-memory could abort with a diagnostic or be unspecified.

**Blocks:** Roadmap Phase 6, and any claim that runtime errors are predictable.

### U8 — Representing absence

12.4 makes a missing dictionary key a runtime error, because the type system cannot express "a value or nothing". The same gap blocks safe parsing and safe lookup generally.

Alternatives: an `Option`-like built-in parameterized type, which is a real type-system feature; multiple return values; a lookup function taking a default value, which needs no new types and covers most cases; keep errors only.

**Blocks:** the standard-library error strategy, section 17.

---

## 22. Document Status

This is the language contract to implement against. None of it is implemented.

Resolve section 21 before, or during, the roadmap phase each entry names. Record every resolution in [DECISIONS.md](DECISIONS.md) and update this document in the same change.

No feature may be advertised as supported until it is implemented and tested.

# Proposal: Networking

**Status:** **Draft, not ratified and not implemented.** Dovik cannot reach a network at all
today: [STDLIB.md](STDLIB.md) has forty builtins and none of them opens a socket. This file is
the design record for what adding one would mean, and it recommends a narrow first step rather
than a sockets API.

## 1. What is actually being asked for

"Can Dovik do web" nearly always means one of three things, and they need different amounts of
language:

| Want | Needs |
| --- | --- |
| Fetch a URL from a script | One blocking request function |
| Serve HTTP | Many concurrent connections, so a concurrency model |
| Run in a browser | A WebAssembly target *and* a JS host |

The third is partly built — `--target wasm` exists ([PROPOSAL-WASM.md](PROPOSAL-WASM.md)) — and
needs a host, not a network stack. The second is blocked behind
[PROPOSAL-CONCURRENCY.md](PROPOSAL-CONCURRENCY.md), which recommends against concurrency until a
real program is core-limited. **The first is the only one that is tractable now**, and it is
also the one a command-line language is actually for.

## 2. The recommendation: one function, not an API

```dovk
body: String? = fetch("https://example.com/data.json")

if body != none:
    print(len(body))
else:
    print("could not fetch")
```

`fetch(url: String) -> String?` — blocking, text only, following redirects, returning `none`
on any failure. That is the whole of it.

**Why `String?` and not a trap.** A network failure is not a program defect; it is the normal
case. [SPEC.md](SPEC.md) 15.2's runtime errors are all defects — overflow, a bad index, a
missing key — and a timed-out request belongs with `parse_int` rather than with them. Optional
types ([SPEC.md](SPEC.md) 23) exist precisely so this does not need a new error mechanism, and
`parse_int` already set the precedent.

**Why text only.** `String` is defined as UTF-8 text ([SPEC.md](SPEC.md) 3.1) and there is no
byte type. `read_file` already made this choice and documents it
([STDLIB.md](STDLIB.md) 5): invalid UTF-8 is replaced rather than rejected. A binary download
needs a `Bytes` type, which is a language change, and no first use case needs one.

**Why no status code, headers, or method.** Each is a second return value, and Dovik has no
tuples and no records. Returning a `Dict<String, String>` would work and would be a worse API
than it looks — the body is not a header. The honest version is: when a program needs the
status code, that program is the argument for designing a response type, and until then
`none` versus a body answers the question that is actually being asked.

## 3. What it costs to build

- **A TLS implementation, or a dependency.** This is the whole difficulty. `http://` is a
  hundred lines of socket code; `https://` is a certificate store, a protocol, and a decade of
  vulnerabilities. Nothing in Dovik has a dependency today ([DECISIONS.md](DECISIONS.md) D029
  declines dependency management outright), and the compiler has zero Rust crates. A `fetch`
  that only speaks `http://` in 2026 is a toy; one that speaks `https://` means linking
  OpenSSL, BearSSL or similar into the runtime and owning that choice.
- **Three runtime functions**, which is the small part: resolve, connect, read.
- **A story for the sandbox.** Under WASI there are no sockets in preview 1, so `fetch` would
  return `none` on the wasm target — a per-target library difference of exactly the kind
  [PROPOSAL-WASM.md](PROPOSAL-WASM.md) 3.3 flagged as undocumented.
- **A test strategy that does not depend on the internet.** CI cannot rely on a remote host
  being up, so this needs a loopback server in the test harness, which is a thread or a
  subprocess, which is awkward in a language with neither.

## 4. The honest recommendation

**Do not build this yet, and for a sharper reason than "it is hard."** The TLS dependency is
the first dependency Dovik would ever take, and taking it for a function nobody has asked for
inverts D029's rule: a dependency arrives when a program wants code it did not write, and no
program here does.

The cheaper thing to say out loud: Dovik is a language for native command-line tools on Linux,
and now for WebAssembly modules. It is not a language for network services, and a `fetch` would
not make it one — that takes the concurrency model that
[PROPOSAL-CONCURRENCY.md](PROPOSAL-CONCURRENCY.md) priced at up to 74% on allocation-heavy code.

## 5. The trigger

A Dovik program that has to read something over a network to do its job, named in writing. Then
this gets designed with that program's needs in front of it, and the TLS decision gets made
with a use case instead of a hypothesis.

## 6. Open decisions

- `http://` only, or TLS from the start. Section 3 argues TLS-or-nothing.
- Whether a `Bytes` type ever exists, which decides whether downloads are possible at all.
- Whether `fetch` is the right shape, or whether the first network thing Dovik gets is a socket
  that a program builds on. One function is easier to remove than an API.

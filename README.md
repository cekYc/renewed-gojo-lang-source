<div align="center">

# ⚡ Zet Lang

### The language that refuses to compile code it doesn't trust.

[![Version](https://img.shields.io/badge/v0.6.0-orange?style=flat-square&label=version)]()
[![License](https://img.shields.io/badge/CC_BY--NC--SA_4.0-red?style=flat-square&label=license)]()
[![Written In](https://img.shields.io/badge/Rust-black?style=flat-square&logo=rust)]()
[![Platform](https://img.shields.io/badge/Windows_%7C_Linux_%7C_macOS-7c3aed?style=flat-square)]()

**Compile-time taint analysis · Native speed · Structured concurrency · Zero runtime overhead**

[Quick Start](#-quick-start) · [Why Zet?](#-why-zet) · [Language Tour](#-language-tour) · [Benchmarks](#-benchmarks) · [Docs](DOCS.md) · [Philosophy](philosophy.md)

</div>

---

### What’s New in v0.6.0?
- **Git package manager**: `zet add`, `remove`, `install`, and `update` manage project dependencies without a central registry.
- **Reproducible resolution**: `zet.lock` pins each direct and transitive package to an immutable Git commit and SHA-256 content checksum.
- **SemVer tags**: Exact versions and ranges resolve against `vX.Y.Z` or `X.Y.Z` repository tags.
- **Shared cache, local checkout**: Git mirrors are reused globally while each project keeps its resolved sources in `.zet/packages/`.
- **Package imports**: Package entry points and modules work with the existing `import` syntax.

### Language features from v0.4
- **Backend First-Class Features**: Automatic HTTP Routing (`@get`, `@post`), Zero-Trust SQLite DB integration.
- **Advanced Error Handling**: `T!` error types, `catch` fallback, and `?` propagation operators.
- **Language Server Protocol (LSP)**: Integrated LSP prototype (`zet-compiler --lsp`) with real-time taint analysis and scope diagnostics.
- **Module System**: `import` system with multi-file project support (`std.http`, `std.db`).
- **Data Structures & Syntax**: Custom structs, array indexing, array mutations, pattern matching (`match`), and quick assignments (`+=`, `-=`).


---

## What is Zet?

**Zet** (Zero Trust) is a compiled programming language where **every piece of external data is untrusted by default** — and the compiler enforces it.

Network responses, user input, file reads — they're all born as `Untrusted` types. You literally cannot use them without passing through a `validate` block. Not at runtime. **At compile time.** Your binary never ships with an unvalidated input path.

Under the hood, Zet compiles through Rust to an optimized native binary — no VM or interpreter is placed between the generated program and the operating system.

```
┌─────────────┐      ┌───────────────────┐      ┌──────────────┐      ┌──────────┐
│  .zt source  │ ──▶  │  Zet Compiler      │ ──▶  │  Rust codegen │ ──▶  │  Binary   │
│              │      │  taint + scope +   │      │  (optimized)  │      │  (native) │
│              │      │  determinism check │      │               │      │           │
└─────────────┘      └───────────────────┘      └──────────────┘      └──────────┘
```

---

## 🤔 Why Zet?

Most languages let you do this:

```python
# Python — runs fine, ships to production, gets hacked
user_input = input("Enter query: ")
db.execute(f"SELECT * FROM users WHERE name = '{user_input}'")  # 💀 SQL Injection
```

In Zet, **this doesn't compile:**

```zet
nondet fn main() -> Void {
    let query = call Console.read("Enter query: ")  // type: Untrusted
    println(query)  // ❌ COMPILE ERROR: tainted variable 'query' used without validation
}
```

You're forced to validate first:

```zet
nondet fn main() -> Void {
    let query = call Console.read("Enter query: ")

    validate query {
        success: {
            // 'query' is now a trusted String — safe to use
            println("User said: " + query)
        }
    }
}
```

This isn't a linter warning. It's not a "best practice." **The compiler won't produce a binary until you handle it.**

### The Four Pillars

| Pillar | What it means | Compile-time enforced? |
|--------|--------------|:---:|
| 🔒 **Zero Trust** | All external data is `Untrusted`. Must `validate` before use. | ✅ |
| ⚡ **Native Speed** | No VM, no GC. Compiles to optimized machine code via Rust. | — |
| 🧠 **Smart Engine** | `det` fns get pure codegen; `nondet` gets async. Mixing them is an error. | ✅ |
| 🧵 **Structured Concurrency** | `spawn` only works inside `scope` blocks. No zombie threads. Ever. | ✅ |

---

## 🚀 Quick Start

> **Requirements:** Windows, Linux, or macOS and a [Rust stable toolchain](https://rustup.rs/) on PATH. Package commands also require [Git](https://git-scm.com/) on PATH.

### Option A — Download the installer
1. Download the package for your platform from [Zet Setup Releases](https://github.com/cekYc/zet-setup/releases).
2. Windows: run `kurulum.bat`. Linux/macOS: run `chmod +x install.sh zet bin/zet-compiler && ./install.sh`.
3. Open a new terminal and run `zet --version`.

### Option B — Build from source
```bash
git clone https://github.com/cekYc/zet-lang-source.git
cd zet-lang-source
cargo build --release --bin zet-compiler
```

### Create and run a project

```bash
zet new hello
cd hello
zet run
zet build
```

`zet new` creates `zet.toml` and `src/main.zt`. `zet build` writes the native executable to `.zet/bin/`. You can still run a standalone file with `zet hello.zt`.

### Add and use a package

```bash
zet add owner/repository@^1.0
zet install
zet update repository
zet remove repository
```

The dependency name comes from the package repository's `zet.toml`. Import it by that name:

```zet
import repository
```

Commit `zet.toml` and `zet.lock`; do not commit `.zet/`. See [the package manager guide](DOCS.md#11-v06-git-paket-yöneticisi) for repository requirements, SemVer rules, cache paths, and transitive dependencies.

---

## 📖 Language Tour

### Variables & Types

```zet
let name = "Zet"
let age = 25
let scores = [100, 95, 87]
let first = scores[0]
```

| Type | Description |
|------|------------|
| `i64` | 64-bit integer |
| `String` | UTF-8 text |
| `Array<T>` | Typed collection |
| `Untrusted` | Tainted external data — cannot be used without `validate` |
| `Void` | No return value |

### Functions: `det` vs `nondet`

Zet forces you to declare your function's purity. The compiler verifies it — and rejects violations:

```zet
// Pure function — CPU & memory only. Async I/O here = compile error.
det fn fibonacci(n: i64) -> i64 {
    if n <= 1 { return n }
    println("Computing...")  // print/println are allowed in det functions
    return fibonacci(n - 1) + fibonacci(n - 2)
}

// Impure function — networking, async I/O, side effects.
nondet fn fetch_data() -> Void {
    let response = call HTTP.get("https://api.example.com/data")
    validate response {
        success: {
            println("Got: " + response)
        }
    }
}
```

> You can also write `deterministic` / `nondeterministic` in full — both forms are accepted.

**Rejected at compile time:**
- Async I/O calls (`HTTP.get`, `Console.read`) inside a `det` function
- `call` keyword on a `det` function (pure functions don't need async)

### Taint Analysis (Zero Trust in Action)

Any data from `Console.read`, `HTTP.get`, or similar sources is `Untrusted`:

```zet
let input = call Console.read("Your name: ")  // type: Untrusted
let data = call HTTP.get("https://...")        // type: Untrusted

// Using 'input' or 'data' directly anywhere = COMPILE ERROR
// You MUST validate:

validate input {
    success: {
        // 'input' is now a clean String — taint removed
        println("Hello, " + input)
    }
}
```

Taint **propagates** — deriving a value from tainted data (JSON parsing, indexing, concatenation) produces another `Untrusted` value.

### Structured Concurrency

```zet
nondet fn main() -> Void {
    scope Network {
        spawn HTTP.get("https://api-1.com")
        spawn HTTP.get("https://api-2.com")
    }
    // Execution reaches here ONLY after ALL spawns in 'Network' have completed.
    // No dangling threads. No fire-and-forget. No zombies.

    println("All network calls done.")
}
```

`spawn` outside a `scope`? **Compile error.** A `scope` block collects every spawned task into a `JoinHandle` vec and awaits all of them before proceeding to the next line.

### The `call` Keyword

`call` awaits a nondeterministic operation inline:

```zet
let now = call Util.now()                // pauses this task, not the whole program
let page = call HTTP.get("https://...")  // async under the hood
let n = call Util.to_int("42")           // string → i64
```

Using `call` on a `det` function is a compile error — pure functions don't need async machinery.

---

## 📊 Build profile

Release builds use Rust’s optimized native-code pipeline with LTO, a single codegen unit, `panic=abort`, and symbol stripping. Performance numbers are intentionally not published until the benchmark suite is reproducible across supported operating systems.

---

## 🔧 Standard Library (v0.2)

| Module | Function | Returns | Description |
|--------|----------|---------|-------------|
| **Built-in** | `print(value)` | `Void` | Print to stdout (no newline) |
| **Built-in** | `println(value)` | `Void` | Print to stdout (with newline) |
| **Console** | `call Console.read(prompt)` | `Untrusted` | Read user input from terminal |
| **HTTP** | `call HTTP.get(url)` | `Untrusted` | Async HTTP GET request |
| **Util** | `call Util.now()` | `i64` | Current Unix timestamp in ms |
| **Util** | `call Util.to_int(s)` | `i64` | Parse string to integer |
| — | `json(data, key)` | `String` | Extract a field from JSON text |

---

## 🏗️ Compiler Architecture

```
src/
├── main.rs              # CLI entry & pipeline orchestrator
├── project.rs           # zet.toml discovery & isolated .zet workspaces
├── package.rs           # Git resolution, SemVer, cache, and zet.lock
├── parser.rs            # Nom-based recursive descent parser
├── ast.rs               # AST node definitions
├── codegen.rs           # Rust code generation (preamble + per-function)
└── analysis/
    ├── taint.rs         # HashSet-based taint tracking & propagation
    ├── determinism.rs   # Purity enforcement with nondeterministic stdlib list
    └── scope.rs         # spawn-inside-scope validation
```

**Pipeline:** `.zt` → Parse → Taint Analysis → Determinism Check → Scope Validation → Rust Codegen → `cargo build` → Native Binary

---

## 🗺️ Roadmap

- [x] Compile-time taint analysis with propagation
- [x] Deterministic / Nondeterministic function enforcement
- [x] Structured concurrency (`scope` + `spawn` + `JoinHandle`)
- [x] HTTP client, JSON parsing, console I/O
- [x] Optimized native Rust codegen
- [x] Pattern matching
- [x] Custom struct types
- [x] Module system & imports
- [x] Windows, Linux, and macOS packages
- [x] LSP diagnostics prototype
- [x] Project manifests and `zet new/run/build` workflow
- [x] Git package manager with SemVer and reproducible locks
- [ ] Central package registry and publishing workflow

---

## 📜 License

[CC BY-NC-SA 4.0](LICENSE) — Free for non-commercial use. Attribution required. Share-alike.

---

<div align="center">

**Zet doesn't trust your inputs. And neither should you.**

*Star the repo, clone it, and try writing something in `.zt` — you might be surprised how different it feels when the compiler actually has your back.*

</div>

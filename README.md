# hulk-compiler

`hulk-compiler` is a compiler for **HULK**, an educational programming language designed to combine a modern, expressive syntax with a robust type system. HULK supports full type inference, structural typing, nominal inheritance, advanced polymorphism, protocols, and iterable abstractions.

This project implements the full pipeline from source code to executable output:

- Lexical analysis
- Parsing
- Semantic analysis
- HIR lowering
- LLVM backend code generation
- Native execution through a C runtime

---

## Features

- **Lexer, parser, semantic analyzer, and backend**
- **Type inference** for unannotated functions and generic templates
- **Protocols / interfaces** with structural typing
- **Generative iterable types** such as `T*`
- **Monomorphization** for generic types and functions
- **Nominal inheritance** and virtual methods

### Built-in Runtime Functions

| Function | Description |
|---|---|
| `sqrt(x)` | Square root |
| `sin(x)` | Sine |
| `cos(x)` | Cosine |
| `exp(x)` | Exponential |
| `log(base, value)` | Logarithm |
| `rand()` | Random number |
| `print(...)` | Print to output |

### Built-in Constants

| Constant | Value |
|---|---|
| `PI` | Pi (π) |
| `E` | Euler's number (e) |

### String Operations

| Operator | Description |
|---|---|
| `@` | String concatenation |
| `@@` | String concatenation with whitespace |

> **Note:** Protocols are desugared during semantic analysis and do not reach the backend directly. Iterables are lowered to basic control-flow constructs before code generation.

---

## Technology Stack

| Component | Technology |
|---|---|
| **Language** | Rust |
| **Lexer** | [`logos`](https://docs.rs/logos) |
| **Parser** | [`chumsky`](https://docs.rs/chumsky) |
| **LLVM backend** | [`inkwell`](https://docs.rs/inkwell) |
| **Diagnostics** | [`ariadne`](https://docs.rs/ariadne) |
| **Serialization / snapshots** | [`serde`](https://serde.rs), [`insta`](https://insta.rs) |
| **Runtime support** | C (`runtime/runtime.c`) |

---

## Repository Structure

```text
.
├── Cargo.lock
├── Cargo.toml
├── Makefile
├── README.md
├── docs
├── runtime
│   └── runtime.c
├── src
│   ├── ast.rs
│   ├── backend
│   │   ├── context.rs
│   │   ├── decl
│   │   │   ├── decl_types.rs
│   │   │   ├── functions.rs
│   │   │   ├── methods.rs
│   │   │   └── mod.rs
│   │   ├── emit.rs
│   │   ├── error.rs
│   │   ├── expr
│   │   │   ├── assign.rs
│   │   │   ├── binary.rs
│   │   │   ├── block.rs
│   │   │   ├── call.rs
│   │   │   ├── control_flow.rs
│   │   │   ├── let_expr.rs
│   │   │   ├── mod.rs
│   │   │   ├── new.rs
│   │   │   ├── postfix.rs
│   │   │   ├── primary.rs
│   │   │   └── unary.rs
│   │   ├── functions.rs
│   │   ├── method_slots.rs
│   │   ├── mod.rs
│   │   ├── runtime.rs
│   │   └── types.rs
│   ├── diagnostics
│   │   ├── diagnostic.rs
│   │   ├── mod.rs
│   │   └── render.rs
│   ├── lexer
│   │   ├── error.rs
│   │   ├── lexer.rs
│   │   ├── mod.rs
│   │   ├── span.rs
│   │   ├── tests.rs
│   │   └── token.rs
│   ├── main.rs
│   ├── parser
│   │   ├── decl
│   │   │   ├── function_decl.rs
│   │   │   ├── mod.rs
│   │   │   ├── protocol_decl.rs
│   │   │   ├── snapshots
│   │   │   └── type_decl.rs
│   │   ├── error.rs
│   │   ├── expr
│   │   │   ├── assign.rs
│   │   │   ├── binary.rs
│   │   │   ├── block.rs
│   │   │   ├── control_flow.rs
│   │   │   ├── let_expr.rs
│   │   │   ├── mod.rs
│   │   │   ├── new.rs
│   │   │   ├── postfix.rs
│   │   │   ├── primary.rs
│   │   │   ├── snapshots
│   │   │   └── unary.rs
│   │   ├── mod.rs
│   │   ├── program.rs
│   │   ├── snapshots
│   │   ├── test_utils.rs
│   │   └── tests.rs
│   └── semantic
│       ├── analyzer.rs
│       ├── builtin.rs
│       ├── context.rs
│       ├── decl
│       │   ├── collect.rs
│       │   ├── declarations.rs
│       │   ├── functions.rs
│       │   ├── inherit.rs
│       │   ├── methods_generic.rs
│       │   ├── mod.rs
│       │   ├── protocols.rs
│       │   ├── register.rs
│       │   ├── resolve_constructor.rs
│       │   ├── types.rs
│       │   └── types_generic.rs
│       ├── error.rs
│       ├── expr
│       │   ├── assign.rs
│       │   ├── binary.rs
│       │   ├── block.rs
│       │   ├── call.rs
│       │   ├── control_flow.rs
│       │   ├── let_expr.rs
│       │   ├── mod.rs
│       │   ├── new.rs
│       │   ├── postfix.rs
│       │   ├── primary.rs
│       │   └── unary.rs
│       ├── hir.rs
│       ├── mod.rs
│       ├── symbols.rs
│       ├── test_utils.rs
│       ├── tests.rs
│       └── types.rs
├── stdlib
│   └── prelude.hulk
└── tests
    ├── recursion.hulk
    ├── render.hulk
    └── ships.hulk
```

---

## Standard Library

The standard library is loaded from:

```
stdlib/prelude.hulk
```

It defines built-in functionality such as:

- `protocol Iterable`
- `type Range`
- `range(start, end)`
- Math and utility helpers
- Built-in protocol/type behavior used by semantic analysis

> Protocols are desugared during compilation and do not reach the LLVM backend as standalone runtime objects.

---

## Build Requirements

This project uses **LLVM 18** through `inkwell` with the `llvm18-1` feature.

You need:

- Rust toolchain
- LLVM 18 development files
- A C compiler (`cc`, `clang`, or `gcc`)
- `llc` available in your `PATH` (or configured through `HULK_LLC`)
- `cc`/`clang`/`gcc` available in your `PATH` (or configured through `HULK_CC`)

### Verify LLVM

```bash
llvm-config --version
```

Expected output:

```
18.x.x
```

---

## Installation Notes for LLVM 18

### Linux (Ubuntu / Debian / WSL)

**1. Install dependencies:**

```bash
sudo apt update
sudo apt install -y \
    build-essential \
    wget \
    curl \
    git \
    cmake \
    ninja-build \
    pkg-config \
    libzstd-dev \
    zlib1g-dev \
    libxml2-dev \
    libffi-dev
```

**2. Install LLVM 18 and the development packages:**

```bash
sudo apt install -y llvm-18 llvm-18-dev libpolly-18-dev
```

**3. If needed, set the environment variables:**

```bash
export LLVM_CONFIG_PATH=/usr/lib/llvm-18/bin/llvm-config
export LLVM_SYS_180_PREFIX=/usr/lib/llvm-18
```

### macOS

**1. Install LLVM 18 with Homebrew:**

```bash
brew install llvm@18
```

**2. Set environment variables:**

```bash
export LLVM_CONFIG_PATH="$(brew --prefix llvm@18)/bin/llvm-config"
export LLVM_SYS_180_PREFIX="$(brew --prefix llvm@18)"
export PATH="$(brew --prefix llvm@18)/bin:$PATH"
```

### Windows

Install the LLVM 18 binary distribution and make sure LLVM is added to `PATH`.

If you use MSVC, also install:

- Desktop development with C++

---

## Build and Run

The normal build/run workflow is:

```bash
make build
./hulk path/to/file.hulk
./output
```

### What These Commands Do

| Command | Description |
|---|---|
| `make build` | Compiles the Rust project in release mode and copies the binary to `./hulk` |
| `./hulk path/to/file.hulk` | Parses, analyzes, lowers, and emits LLVM IR / native artifacts for the given HULK file |
| `./output` | Runs the generated native executable |

### Other Useful Commands

**Clean the project:**

```bash
make clean
```

This removes:

- The compiled `hulk` binary
- Generated object files
- Emitted LLVM IR files
- Generated native output

**Run tests:**

```bash
cargo test
```

This runs the lexer, parser, semantic, and snapshot tests.

---

## Language Examples

### Arithmetic and Built-ins

```hulk
print(sin(2 * PI) ^ 2 + cos(3 * PI / log(4, 64)));
```

### Inline Functions

```hulk
function pi() => PI;
```

### Function Bodies

```hulk
function ackermann_pi(m: Number, n: Number): Number {
    (
    if (m == 0) n + 1;
    elif (m > 0 & n == 0) ackermann(m - 1, 1)
    else ackermann(m - 1, ackermann(m, n - 1))
    ) * PI;
}
```

### `let`

```hulk
let number: Number = PI, text: String = "The meaning of life is" in
    print(text @ number);
```

### Blocks

```hulk
{
    print("PI");
    print(PI);
    print("E");
    print(E);
}
```

### `while`

```hulk
while (true) print(PI);
```

### `if` / `elif` / `else`

```hulk
if (a % 3 == 0) print(PI) elif (a % 3 == 1) print(E) else print(E ^ PI);
```

### `for` over Iterables

```hulk
for (x in range(0, 10)) print(PI ^ x);
```

> The `for` loop is lowered to explicit iterator-style control flow during compilation and works with iterable protocols.

### Types and Methods

```hulk
type Point(x, y) {
    x = x;
    y = y;

    getX() => self.x;
    getY() => self.y;

    setX_PI() => self.x := PI;
    setY_PI() => self.y := PI;
}
```

### Inheritance and Polymorphism

```hulk
type PIE(pi, e) {
    pi = pi;
    e = e;

    pie() => self.pi ^ self.e;
}

type life inherits PIE {
    pie() => "life is" @@ base();
}

let pi_e = new life(PI, E) in
    print(p.pie());
```

### Type Checks and Casts

```hulk
let p: PIE = new life() in {
    if (p is life) (p as life).pie()
    else "life is pi ^ e";
}
```

### Protocols / Interfaces

```hulk
protocol PIrotocol {
    pi(): Number;
}

interface Erotocol {
    e(): Number;
}

protocol PIErotocol extends PIrotocol, Erotocol {
    pie(pi: Number, e: Number): Number
}
```

---

## Tests

Run all tests with:

```bash
cargo test
```

The `tests/` directory contains example HULK programs used to validate parsing, semantic checks, and runtime behavior.


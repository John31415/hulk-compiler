# hulk-compiler

About
A high-performance HULK language compiler in Rust, featuring a custom lexer, parser, semantic analyzer, and LLVM backend.

## Prerequisites: LLVM 20 Installation Guide

This project uses **Inkwell** and **llvm-sys (v201.x)**, which require **LLVM 20** and its development libraries. Installing a different LLVM version will usually result in compilation or linking errors.

Follow the instructions for your operating system.

---

### Linux (Ubuntu / Debian / WSL)

If you previously installed other LLVM versions, it is recommended to remove them first to avoid library conflicts.

#### Remove Existing LLVM Installations

```bash
sudo apt remove --purge -y "llvm*" "clang*" "libllvm*" "polly*"
sudo apt autoremove -y
sudo apt clean
```

#### Install Required Dependencies

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

#### Install LLVM 20

```bash
wget https://apt.llvm.org/llvm.sh
chmod +x llvm.sh
sudo ./llvm.sh 20
```

#### Install LLVM Development Packages

```bash
sudo apt install -y llvm-20-dev libpolly-20-dev
```

#### Configure Environment Variables

Create a stable symlink for `llvm-config`:

```bash
sudo ln -sf /usr/lib/llvm-20/bin/llvm-config /usr/bin/llvm-config
```

Add the required environment variables to your shell profile:

```bash
echo 'export LLVM_CONFIG_PATH=/usr/lib/llvm-20/bin/llvm-config' >> ~/.bashrc
echo 'export LLVM_SYS_201_PREFIX=/usr/lib/llvm-20' >> ~/.bashrc
```

Apply the changes:

```bash
source ~/.bashrc
```

---

### macOS

LLVM can be installed using Homebrew.

#### Install LLVM 20

```bash
brew install llvm@20
```

#### Configure Environment Variables

Add the following lines to your shell profile (`~/.zshrc` or `~/.bash_profile`):

```bash
export LLVM_CONFIG_PATH=$(brew --prefix llvm@20)/bin/llvm-config
export LLVM_SYS_201_PREFIX=$(brew --prefix llvm@20)
export PATH="$(brew --prefix llvm@20)/bin:$PATH"
```

Apply the changes:

```bash
source ~/.zshrc
```

---

### Windows

#### Install LLVM 20

Download and install the official LLVM 20 x64 binary release:

* `LLVM-20.x.x-win64.exe`

During installation, make sure to enable:

> Add LLVM to the system PATH

#### Configure Environment Variables

Open **PowerShell as Administrator** and run:

```powershell
[Environment]::SetEnvironmentVariable(
    "LLVM_SYS_201_PREFIX",
    "C:\Program Files\LLVM",
    "User"
)
```

If LLVM was installed in a different directory, replace the path accordingly.

#### Install Visual Studio Build Tools

Rust's MSVC toolchain requires the Visual C++ compiler.

Using the **Visual Studio Installer**, install:

* Desktop development with C++

---

### Verify Installation

Ensure the correct LLVM version is being used:

```bash
llvm-config --version
```

Expected output:

```text
20.x.x
```

---

### Build the Project

Before building, remove any stale build artifacts:

```bash
cargo clean
rm -rf target Cargo.lock
```

Compile the project:

```bash
cargo build
```

If everything is configured correctly, Cargo should successfully compile both `llvm-sys` and `inkwell`.

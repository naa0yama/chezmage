# chezmage

![coverage](https://raw.githubusercontent.com/naa0yama/chezmage/badges/coverage.svg)
![test execution time](https://raw.githubusercontent.com/naa0yama/chezmage/badges/time.svg)

chezmoi + age encryption with **GPG (YubiKey) protected age secret keys that never touch disk**. A single Rust binary.

> **chezmage** `/ʃeɪ meɪdʒ/` (shay-mage) — a portmanteau of [chezmoi](https://www.chezmoi.io/) and [age](https://age-encryption.org/).
>
> The name also evokes "mage" (wizard): your secret keys materialize in memory for a single invocation, then vanish without a trace — as if by magic.

## Features

- **Single binary** — no symlinks or shell scripts needed
- **Auto-reads chezmoi.toml** — discovers `identity` / `identities` key paths
- **Multiple key support** — GPG encrypted / plaintext / mixed
- **Zero disk writes** — keys exist only in process memory (env var)
- **Security hardened** — mlock, zeroize, core dump disable, ptrace block
- **Cross-platform** — Linux / Windows

## Modes

chezmage operates in two modes from a single binary:

| Mode    | Invocation                   | Purpose                                                  |
| ------- | ---------------------------- | -------------------------------------------------------- |
| Wrapper | `chezmage <args>`            | Decrypt GPG keys, set env var, then exec `chezmoi`       |
| Shim    | `chezmage --shim <age-args>` | Feed the in-memory key to `age` via a pipe (no disk I/O) |

**Wrapper mode** is the user-facing entry point. It reads `chezmoi.toml`, decrypts GPG-encrypted identity files (triggering a single YubiKey touch), loads the age secret keys into memory, and execs the real `chezmoi` binary.

**Shim mode** is called by chezmoi internally. When chezmoi needs to decrypt an `.age` file, it invokes the `command` configured in `[age]` — which is `chezmage --shim`. The shim reads the key from the `CHEZMOI_AGE_KEY` env var (set by wrapper mode), writes it to a pipe, and execs the real `age` binary with the pipe as the identity source. This avoids writing keys to disk.

## How it works

```
chezmage apply                    ← Wrapper mode
       │
       ├─ Read chezmoi.toml
       │    [age] identity = "~/.config/chezmoi/key1.gpg"
       │    [age] identities = ["key2.gpg", "key3.txt"]
       │
       ├─ Process each identity:
       │    *.gpg / *.asc → gpg --decrypt (YubiKey touch)
       │    other         → read file
       │    /dev/null, NUL → skip
       │
       ├─ Combine all keys → $CHEZMOI_AGE_KEY (mlock'd memory)
       │
       └─ exec chezmoi apply
              │
              ├─ file1.age → chezmage --shim
              │    └─ echo $CHEZMOI_AGE_KEY | age -d -i - file1.age
              ├─ file2.age → chezmage --shim
              └─ ...
              GPG calls: once per key file
              YubiKey touch: once within gpg-agent cache
```

## Security

| Measure                  | Implementation                                   |
| ------------------------ | ------------------------------------------------ |
| mlock() / VirtualLock()  | Prevent key memory from being swapped            |
| zeroize (Zeroizing\<T\>) | Zero memory on drop                              |
| RLIMIT_CORE = 0          | Disable core dumps                               |
| PR_SET_DUMPABLE = 0      | Block ptrace + protect /proc/PID/environ (Linux) |
| No disk writes           | Keys exist only in process memory                |
| Process exit = cleanup   | SIGKILL included — env var vanishes with process |

## Build

```bash
# Debug build
mise run build

# Release build
cargo build --release

# With OpenTelemetry support
cargo build --features otel
```

## Install

### mise (recommended)

Install with a single command using [mise](https://mise.jdx.dev/):

```bash
mise use -g github:naa0yama/chezmage
```

Pin a specific version:

```bash
mise use -g github:naa0yama/chezmage@0.1.3
```

### Pre-built binaries

Download from [GitHub Releases](https://github.com/naa0yama/chezmage/releases/latest):

| Platform | Architecture  | File                                                   |
| -------- | ------------- | ------------------------------------------------------ |
| Linux    | x86_64        | `chezmage-v{VERSION}-x86_64-unknown-linux-gnu.tar.gz`  |
| Linux    | aarch64       | `chezmage-v{VERSION}-aarch64-unknown-linux-gnu.tar.gz` |
| macOS    | Apple Silicon | `chezmage-v{VERSION}-aarch64-apple-darwin.tar.gz`      |
| Windows  | x86_64        | `chezmage-v{VERSION}-x86_64-pc-windows-msvc.zip`       |

```bash
# Example: Linux x86_64
curl -fsSL https://github.com/naa0yama/chezmage/releases/latest/download/chezmage-v{VERSION}-x86_64-unknown-linux-gnu.tar.gz | tar xz
install -m 755 chezmage ~/.local/bin/
```

### Build from source

```bash
cargo install --git https://github.com/naa0yama/chezmage.git
```

## Setup

### 1. Generate age key and encrypt with GPG

```bash
# Create key storage directory
mkdir -p ~/.config/chezmoi

# Generate age key pair
age-keygen --output ~/.config/chezmoi/age-personal.key
# Copy the output string "Public key: age1..."

# Encrypt the generated private key with GPG (YubiKey)
gpg --armor --encrypt --recipient "your-email@example.com" \
  --output ~/.config/chezmoi/age-personal.asc \
  ~/.config/chezmoi/age-personal.key

# Securely delete the plaintext key
shred -u ~/.config/chezmoi/age-personal.key
```

### 2. Configure chezmoi.toml

```toml
encryption = "age"

[age]
command = "chezmage"
args = ["--shim"]
identities = [
	"~/.config/chezmoi/age-personal.asc",
]
recipients = [
	"age1xxxx...",
]
```

### 3. Use

```bash
chezmage apply
chezmage diff
chezmage add --encrypt ~/.ssh/config

# Recommended alias
alias chezmoi='chezmage'
```

## Smart GPG decryption skip

chezmage skips GPG decryption (and YubiKey touch) when it is not needed:

**Passthrough subcommands** — commands that never read encrypted content skip decryption entirely. This includes metadata-only commands and write-only encryption commands that use only the age public key (recipient):

`add`, `age-keygen`, `cat-config`, `cd`, `chattr`, `completion`, `data`, `doctor`, `dump-config`, `edit-config`, `edit-config-template`, `encrypt`, `execute-template`, `forget`, `generate`, `git`, `help`, `ignored`, `license`, `managed`, `purge`, `re-add`, `secret`, `source-path`, `state`, `target-path`, `unmanaged`, `upgrade`

**`--exclude encrypted`** — when you explicitly exclude encrypted entries, chezmoi will not decrypt any files, so chezmage skips GPG decryption as well:

```bash
# No YubiKey touch required — encrypted files are excluded
chezmage status --exclude encrypted
chezmage apply -x encrypted
chezmage diff --exclude=dirs,encrypted
```

The `--include` flag (`-i`) can re-enable encrypted entries. chezmage evaluates flags left-to-right with the last flag winning, matching chezmoi semantics. Unknown or ambiguous flags always fall back to decryption (safe default).

## Identity discovery priority

1. **chezmoi.toml** `[age] identity` / `identities` (excluding `/dev/null`, `NUL`)
2. **Environment variable** `CHEZMOI_AGE_GPG_KEY_FILE` (comma/semicolon separated)
3. **Auto-scan** config directories for `*.gpg` / `*.asc` files

## Environment variables

| Variable                   | Purpose                                        |
| -------------------------- | ---------------------------------------------- |
| `CHEZMOI_AGE_GPG_KEY_FILE` | Key file path(s) as fallback (comma separated) |
| `CHEZMOI_AGE_KEY`          | (Internal) Pre-decrypted key. Skips GPG if set |
| `CHEZMOI_CONFIG`           | Override chezmoi config file path              |
| `XDG_CONFIG_HOME`          | Override config search path                    |

## Development

```bash
mise run test          # All tests (unit + integration)
mise run pre-commit    # fmt:check + clippy:strict + ast-grep
mise run coverage      # Coverage report
```

## Troubleshooting

### Linux

```bash
RUST_LOG=trace RUST_BACKTRACE=1 chezmage --debug diff
chezmage -V
```

### Windows

```powershell
$env:RUST_LOG="trace"
$env:RUST_BACKTRACE=1
chezmage --debug diff
chezmage -V
```

## License

AGPL-3.0

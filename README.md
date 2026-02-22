# chezmage

chezmoi + age encryption with **GPG (YubiKey) protected age secret keys that never touch disk**. A single Rust binary.

## Features

- **Single binary** — no symlinks or shell scripts needed
- **Auto-reads chezmoi.toml** — discovers `identity` / `identities` key paths
- **Multiple key support** — GPG encrypted / plaintext / mixed
- **Zero disk writes** — keys exist only in process memory (env var)
- **Security hardened** — mlock, zeroize, core dump disable, ptrace block
- **Cross-platform** — Linux / Windows

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

```bash
install -m 755 target/release/chezmage ~/.local/bin/
```

## Setup

### 1. Generate age key and encrypt with GPG

```bash
age-keygen -o /tmp/age-key.txt
# Public key: age1xxxx...

gpg -e -r YOUR_GPG_KEY_ID -o ~/.config/chezmoi/age-key.gpg /tmp/age-key.txt
shred -u /tmp/age-key.txt
```

### 2. Configure chezmoi.toml

```toml
encryption = "age"

[age]
identity = "~/.config/chezmoi/age-key.gpg"
command = "chezmage"
args = ["--shim"]
recipient = "age1xxxx..."
```

See [examples/dot_chezmoi.toml.tmpl](./examples/dot_chezmoi.toml.tmpl) for a full template.

### 3. Use

```bash
chezmage apply
chezmage diff
chezmage add --encrypt ~/.ssh/config

# Recommended alias
alias chezmoi='chezmage'
```

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

```bash
RUST_LOG=trace RUST_BACKTRACE=1 cargo run -- help
```

## License

AGPL-3.0

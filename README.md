# muaalem-engine
بفضل الله وحده لا شريك له:  محرك المعلم القرآني لتصحيح  تلاوة القرآن الكريم من حروف وصفات وتجويد يعمل في أي مكان وب SKD ل Python, Kotlin, Swift عال الأداء بلغة ال Rust

## Embedded Quran data assets

The `quran-muaalem-transcript` crate embeds its Quran JSON assets with
`include_str!`. This keeps the SDK self-contained, so applications do not need
to install or locate separate data files at runtime.

The three embedded files total **3,356,048 bytes (3.20 MiB)**:

- `begin_with_hamzat_wasl.json`: 11,267 bytes
- `quran-alphabet.json`: 10,607 bytes
- `quran-uthmani-imlaey.json`: 3,334,174 bytes

`quran-uthmani-imlaey-map.json` (13,491,416 bytes) ships in `assets/` but is
not embedded into the binary.

## BoltFFI bindings

The `quran-muaalem-engine` crate exports its SDK API with [BoltFFI](https://boltffi.dev).
Every binding has a smoke test in [`crates/engine/tests/bindings/`](crates/engine/tests/bindings/):
it packs the real native library and calls `version()` from that language.
All commands below run from the repo root; `make help` lists them.

### Quick start

```sh
make install-boltffi     # the boltffi CLI version pinned in Cargo.toml
make bind-python         # download + build official CPython 3.15.2 into .toolchains/ (no sudo)
make bind-all            # ... or every language SDK this host can test (includes Python)
make doctor              # check every toolchain this machine can test; prints fixes
make test-bind-all       # pack + smoke-test every binding this machine can run
```

The Python binding never uses the system interpreter: `make bind-python` downloads the
official CPython 3.15.2 tarball, verifies its SHA-256, and builds it into
`.toolchains/python-3.15.2`. Install the build headers listed below first; the source build
takes several minutes. Set `PYTHON=/path/to/python3` to use a different interpreter, but it
must be 3.15.2 or newer with `venv`, `pip`, `ssl`, `zlib`, and development headers.

Before the first run, install the system packages for your OS (see below). `make bind-all`
never calls `sudo`; it only installs SDKs into `.toolchains/`. Run `make doctor` afterwards
if a test still fails its pre-flight check.

Run one language with `make test-bind-<lang>`. It runs `make doctor-<lang>` first, so a missing
toolchain fails fast with the fix instead of after a full release build.

### Requirements

`make bind-<lang>` installs the language SDK columns below. The system prerequisites must come
from your package manager.

| Binding | System prerequisites | `make bind-<lang>` installs (into `.toolchains/`, no sudo) |
|---|---|---|
| Python | a C compiler (`cc`) and zlib/OpenSSL development headers (see below) | official CPython 3.15.2, built from source into `.toolchains/python-3.15.2` |
| Java (JVM) | a full **JDK** (`javac` + `JAVA_HOME`), not just a JRE; `clang` | Temurin JDK 21 |
| Kotlin | JDK, `clang`, `kotlinc`, `rustup`, and the Android NDK (`ANDROID_NDK_HOME`) | JDK, kotlinc 2.2.20 and NDK r27c (whichever are missing) |
| Dart | Dart SDK ≥ 3.10.8 | Dart 3.13.5 |
| Swift | macOS 13+ with Xcode (Swift and clang) | nothing (Xcode comes from the App Store) |

All bindings also need Rust (`rustup`) and the `boltffi` CLI (`make install-boltffi`).
`make bind-*` downloads archives, so it also needs `curl`, `unzip` and `tar`.

### System packages

`make bind-*` does not install system packages and never calls `sudo`. Install what you need first:

```sh
# Debian / Ubuntu
sudo apt install build-essential clang zlib1g-dev libssl-dev libffi-dev \
  libbz2-dev libreadline-dev libsqlite3-dev liblzma-dev libzstd-dev curl unzip

# Fedora / RHEL
sudo dnf install gcc make clang zlib-devel openssl-devel libffi-devel \
  bzip2-devel readline-devel sqlite-devel xz-devel libzstd-devel curl unzip

# Arch
sudo pacman -S base-devel clang zlib openssl libffi bzip2 readline sqlite xz zstd curl unzip

# openSUSE
sudo zypper install gcc make clang zlib-devel libopenssl-devel libffi-devel \
  libbz2-devel readline-devel sqlite3-devel xz-devel libzstd-devel curl unzip

# macOS
xcode-select --install   # clang + C toolchain
brew install openssl@3   # CPython's ssl module needs OpenSSL 3
```

Only `zlib` and OpenSSL are required to build a pip-capable Python; the other `-devel` packages
enable optional standard-library modules (`ctypes`, `bz2`, `sqlite3`, `lzma`, `zstd`).
`make bind-python` logs the build to `.toolchains/python-3.15.2-build.log` and prints the log
tail if it fails.

`make doctor` prints the exact command for the package manager it detects.

`make bind-all` installs every language SDK this machine can test without sudo. It does **not**
install `clang`, `gcc`, or the Python build headers; run `make doctor` after it to see what is
still missing.

### Troubleshooting

- `python packaging requires python3 or python in PATH`: BoltFFI could not use the built
  interpreter. Run `make bind-python`, then `make test-bind-python`.
- `missing zlib/OpenSSL development headers`: install the packages listed above and re-run
  `make bind-python`.
- `clang not found`: install clang. On Fedora: `sudo dnf install clang`; on Debian/Ubuntu:
  `sudo apt install clang`.
- `the Swift test ... needs macOS`: expected on Linux. `make test-bind-all` skips Swift on Linux;
  CI runs the Swift test on macOS.

### What's tested

| Binding | What the smoke test proves |
|---|---|
| Python | the wheel builds, installs into a fresh venv, and `version()` works |
| Java | the JNI library builds and `version()` works |
| Kotlin | **desktop JVM only**: the Android Kotlin bindings run through BoltFFI's `desktop_pack` on a normal JVM, not on Android |
| Dart | the host platform's native library only |
| Swift | the **macOS slice only**; no iOS device/simulator slices |

Not tested yet: wasm, the other Android architectures, iOS, and running on a real device or emulator.

Dart, Swift and Kotlin pass `run.sh` an `--overlay` config that builds only what the test needs.
`crates/engine/boltffi.toml` itself keeps every release target.
Packaged output lands in `crates/engine/dist/<target>/`, except Dart, which goes to
`crates/engine/tests/bindings/dart/generated/` (the generated package needs a Dart workspace root).

### CI

`.github/workflows/bindings.yml` runs `make test-bind-<lang>` for each binding: Swift on macOS, the
rest on Ubuntu. CI builds CPython 3.15.2 with `make bind-python` on Linux and caches
`.toolchains/python-3.15.2`; the Swift job runs natively on macOS. Java and Dart come from their
official `actions/setup-*` actions, while `make bind-kotlin` provides `kotlinc`. The `boltffi`
CLI version comes from `Cargo.toml` in both places, so a version bump is a one-file change.
`.github/workflows/ci.yml` runs `make lint` and `make test`.

CI runs on Linux and macOS only; Windows isn't tested.

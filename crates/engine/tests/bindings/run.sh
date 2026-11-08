#!/usr/bin/env bash
# Pack each BoltFFI binding and run its smoke test against the real native library.
#
# Usage: crates/engine/tests/bindings/run.sh [python] [java] [kotlin] [dart] [swift]
#        (default: python java dart, plus swift on macOS)
#
# Prefer the root Makefile: `make test-bind-<lang>` runs `make doctor-<lang>` first and puts
# toolchains installed by `make bind-<lang>` on PATH. Requirements are listed in README.md
# ("BoltFFI bindings"); `make doctor` checks them. Tested in CI on Linux and macOS only.
set -euo pipefail

cd "$(dirname "$0")/../.."
tests=tests/bindings

ROOT="$(cd ../.. && pwd)"
TC_DIR="$ROOT/.toolchains"
PYTHON_VERSION=3.15.2 # keep in sync with toolchains.sh
PYTHON_BIN="$TC_DIR/python-$PYTHON_VERSION/bin/python3"

ENGINE_VERSION="$(cargo pkgid | sed 's/.*[@#]//')"
export ENGINE_VERSION

case "${OSTYPE:-}" in
    msys* | cygwin* | win32*) classpath_sep=";" ;;
    *) classpath_sep=":" ;;
esac

python_interpreter() {
    if [ -n "${PYTHON:-}" ]; then
        if [ ! -x "$PYTHON" ] && ! command -v "$PYTHON" >/dev/null 2>&1; then
            echo "error: PYTHON=$PYTHON is not an executable interpreter" >&2
            return 1
        fi
        printf '%s\n' "$PYTHON"
    elif [ -x "$PYTHON_BIN" ]; then
        printf '%s\n' "$PYTHON_BIN"
    else
        echo "error: Python $PYTHON_VERSION is not installed at $PYTHON_BIN" >&2
        echo "       run 'make bind-python' first" >&2
        return 1
    fi
}

run_python() {
    local python_cmd
    python_cmd="$(python_interpreter)"
    boltffi pack python --release --python "$python_cmd"

    local venv="$tests/python/.venv"
    "$python_cmd" -m venv "$venv"
    local py="$venv/bin/python"
    [ -x "$py" ] || py="$venv/Scripts/python.exe"

    "$py" -m pip install --quiet --force-reinstall dist/python/wheelhouse/*.whl
    "$py" -m unittest discover -s "$tests/python" -v
}

run_java() {
    boltffi pack java --release

    local classes=dist/java-test-classes
    rm -rf "$classes"
    local sources=()
    while IFS= read -r -d '' f; do sources+=("$f"); done < <(find dist/java -name '*.java' -print0)
    javac -d "$classes" "${sources[@]}" "$tests/java/VersionTest.java"

    local native_dirs
    native_dirs="$(find dist/java/native -mindepth 1 -maxdepth 1 -type d | paste -sd "$classpath_sep" -)"
    java -cp "$classes${classpath_sep}dist/java" -Djava.library.path="$native_dirs" VersionTest
}

run_kotlin() {
    # Runs the Android Kotlin bindings on the desktop JVM via BoltFFI's desktop_pack.
    # Android needs at least one architecture, so build just x86_64 (needs ANDROID_NDK_HOME).
    rustup target add x86_64-linux-android
    local overlay=dist/kotlin-desktop.toml
    mkdir -p dist
    cat > "$overlay" <<EOF
[targets.android]
architectures = ["x86_64"]

[targets.android.kotlin.desktop_pack]
enabled = true
EOF
    boltffi pack android --release --overlay "$overlay"

    local jar=dist/kotlin-test.jar
    local sources=()
    while IFS= read -r -d '' f; do sources+=("$f"); done < <(find dist/android/kotlin -name '*.kt' -print0)
    kotlinc "${sources[@]}" "$tests/kotlin/VersionTest.kt" -include-runtime -d "$jar"

    local native_dirs
    native_dirs="$(find dist/android/desktopJniLibs -mindepth 1 -maxdepth 1 -type d | paste -sd "$classpath_sep" -)"
    LD_LIBRARY_PATH="$native_dirs${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}" \
        java -Djava.library.path="$native_dirs" -cp "$jar" VersionTestKt
}

dart_host_target() {
    local os arch
    case "$(uname -s)" in
        Linux) os=linux ;;
        Darwin) os=macos ;;
        MINGW* | MSYS* | CYGWIN*) os=windows ;;
        *) echo "unsupported OS: $(uname -s)" >&2; return 1 ;;
    esac
    case "$(uname -m)" in
        x86_64 | amd64) arch=x86_64 ;;
        aarch64 | arm64) arch=arm64 ;;
        *) echo "unsupported arch: $(uname -m)" >&2; return 1 ;;
    esac
    echo "$os:$arch"
}

run_dart() {
    # By default Dart packs every platform; the smoke test only needs the host.
    local overlay=dist/dart-host.toml
    mkdir -p dist
    printf '[targets.dart]\nnative_targets = ["%s"]\n' "$(dart_host_target)" > "$overlay"
    boltffi pack dart --release --overlay "$overlay"

    (cd "$tests/dart" && dart pub get && dart test)
}

run_swift() {
    # By default Apple packs iOS device + simulator slices only; the smoke test
    # runs natively on macOS, so build just the host macOS slice.
    local arch
    case "$(uname -m)" in
        arm64) arch=arm64 ;;
        x86_64) arch=x86_64 ;;
        *) echo "unsupported arch: $(uname -m)" >&2; return 1 ;;
    esac
    local overlay=dist/apple-host.toml
    mkdir -p dist
    cat > "$overlay" <<EOF
[targets.apple]
include_macos = true
ios_architectures = []
simulator_architectures = []
macos_architectures = ["$arch"]
EOF
    boltffi pack apple --release --overlay "$overlay"

    (cd "$tests/swift" && swift test)
}

langs=("$@")
if [ ${#langs[@]} -eq 0 ]; then
    langs=(python java dart)
    [ "$(uname -s)" = Darwin ] && langs+=(swift)
fi

for lang in "${langs[@]}"; do
    echo "==> $lang"
    "run_$lang"
done

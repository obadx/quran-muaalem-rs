#!/usr/bin/env bash
# Toolchain helper for the BoltFFI binding smoke tests. Called by the root Makefile:
#
#   toolchains.sh install-boltffi        install the CLI pinned by Cargo.toml ([workspace.dependencies] boltffi)
#   toolchains.sh install <lang>|all     download a language toolchain into .toolchains/ (no sudo)
#   toolchains.sh doctor [cli] [<lang>...] pre-flight checks (default: all this host can test); prints fixes
#   toolchains.sh run <cmd...>           run a command with .toolchains/ on PATH (JAVA_HOME, ANDROID_NDK_HOME, ...)
#
# Languages: python java kotlin dart swift. Downloads support Linux and macOS (x86_64/arm64).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../../.." && pwd)"
TC_DIR="$ROOT/.toolchains"

# Pinned toolchain versions (CI installs the same ones where it downloads them).
KOTLIN_VERSION=2.2.20
DART_VERSION=3.13.5
DART_MIN_VERSION=3.10.8 # the generated pubspec requires sdk ^3.10.8
JDK_MAJOR=21
NDK_VERSION=r27c
# Official CPython, built from source into .toolchains/. The interpreter is never taken
# from the system: pip needs development headers and a compiler for the wheel's C bridge.
PYTHON_VERSION=3.15.2
PYTHON_MIN_VERSION=3.15.2
PYTHON_SHA256=f74e1db0eabd651cd01e4e2a00201bd41866abe74869ab716f9bf33cbbaa0943
PYTHON_SOURCE_URL="https://www.python.org/ftp/python/$PYTHON_VERSION/Python-$PYTHON_VERSION.tgz"
PYTHON_PREFIX="$TC_DIR/python-$PYTHON_VERSION"
PYTHON_BIN="$PYTHON_PREFIX/bin/python3"

boltffi_version() {
    sed -n 's/^boltffi = "\(.*\)"/\1/p' "$ROOT/Cargo.toml"
}

# --- environment ----------------------------------------------------------------

setup_env() {
    mkdir -p "$TC_DIR/bin"
    export PATH="$TC_DIR/bin:$PATH"
    # Default to the downloaded CPython; never fall back to the system interpreter.
    # An explicit PYTHON=... still wins (see README).
    if [ -x "$PYTHON_BIN" ]; then
        export PATH="$PYTHON_PREFIX/bin:$PATH"
        if [ -z "${PYTHON:-}" ]; then
            export PYTHON="$PYTHON_BIN"
        fi
    fi
    if [ -d "$TC_DIR/jdk" ]; then
        export JAVA_HOME="$TC_DIR/jdk"
        export PATH="$JAVA_HOME/bin:$PATH"
    elif [ -z "${JAVA_HOME:-}" ] && [ -x /usr/libexec/java_home ] && /usr/libexec/java_home >/dev/null 2>&1; then
        JAVA_HOME="$(/usr/libexec/java_home)" # macOS: /usr/bin/javac is only a stub
        export JAVA_HOME
    elif [ -z "${JAVA_HOME:-}" ] && command -v javac >/dev/null; then
        # boltffi needs JAVA_HOME for the JNI headers; derive it from javac
        JAVA_HOME="$(dirname "$(dirname "$(realpath "$(command -v javac)")")")"
        export JAVA_HOME
    fi
    [ -d "$TC_DIR/kotlinc" ] && export PATH="$TC_DIR/kotlinc/bin:$PATH"
    [ -d "$TC_DIR/dart-sdk" ] && export PATH="$TC_DIR/dart-sdk/bin:$PATH"
    if [ -z "${ANDROID_NDK_HOME:-}" ] && [ -d "$TC_DIR/ndk" ]; then
        export ANDROID_NDK_HOME="$TC_DIR/ndk"
    fi
    return 0
}

host_os() {
    case "$(uname -s)" in
        Linux) echo linux ;;
        Darwin) echo macos ;;
        *) echo unsupported ;;
    esac
}

host_arch() {
    case "$(uname -m)" in
        x86_64 | amd64) echo x86_64 ;;
        aarch64 | arm64) echo arm64 ;;
        *) echo unsupported ;;
    esac
}

require_supported_host() {
    if [ "$(host_os)" = unsupported ] || [ "$(host_arch)" = unsupported ]; then
        echo "error: toolchain downloads support Linux and macOS only ($(uname -s) $(uname -m)); install it manually" >&2
        exit 1
    fi
}

download() { # url dest
    echo "  downloading $1"
    curl -fL --progress-bar -o "$2" "$1"
}

version_ge() { # $1 >= $2, dotted versions
    local IFS=.
    local -a a=($1) b=($2)
    local i
    for i in 0 1 2; do
        ((10#${a[i]:-0} > 10#${b[i]:-0})) && return 0
        ((10#${a[i]:-0} < 10#${b[i]:-0})) && return 1
    done
    return 0
}

cpu_count() { # portable across Linux and macOS (macOS has no nproc)
    if command -v nproc >/dev/null 2>&1; then
        nproc
    elif command -v sysctl >/dev/null 2>&1; then
        sysctl -n hw.ncpu
    else
        getconf _NPROCESSORS_ONLN 2>/dev/null || echo 1
    fi
}

sha256_file() { # file -> hex digest
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$1" | awk '{print $1}'
    elif command -v shasum >/dev/null 2>&1; then
        shasum -a 256 "$1" | awk '{print $1}'
    else
        echo "need sha256sum or shasum to verify the Python tarball" >&2
        return 1
    fi
}

# --- doctor ---------------------------------------------------------------------

failures=0
ok() { printf '  \033[32m✓\033[0m %s\n' "$1"; }
bad() { # message fix
    printf '  \033[31m✗\033[0m %s\n      fix: %s\n' "$1" "$2"
    failures=$((failures + 1))
}

check_cli() {
    echo "boltffi CLI"
    local want have
    want="$(boltffi_version)"
    command -v cargo >/dev/null && ok "cargo" || bad "cargo not found" "install Rust from https://rustup.rs"
    if ! command -v boltffi >/dev/null; then
        bad "boltffi not found (want $want)" "make install-boltffi"
        return
    fi
    have="$(boltffi --version | awk '{print $2}')"
    if [ "$have" = "$want" ]; then
        ok "boltffi $have (matches Cargo.toml)"
    else
        bad "boltffi $have, but Cargo.toml pins $want" "make install-boltffi"
    fi
}

system_pkg_hint() { # apt_package dnf_package [macos_hint] -> install command for this OS
    local apt_pkg="${1:?}" dnf_pkg="${2:-$1}" macos_hint="${3:-xcode-select --install}"
    if [ "$(host_os)" = macos ]; then
        echo "$macos_hint"
    elif command -v dnf >/dev/null 2>&1; then
        echo "sudo dnf install $dnf_pkg"
    elif command -v apt-get >/dev/null 2>&1; then
        echo "sudo apt install $apt_pkg"
    else
        echo "install $apt_pkg (or your distro's equivalent)"
    fi
}

check_c_compiler() { # native builds and the Python wheel's C extension need a compiler/linker
    [ "$(host_os)" = unsupported ] && return # e.g. Windows: the toolchain uses MSVC, not cc
    command -v cc >/dev/null && ok "C compiler: $(command -v cc)" || bad "no C compiler (cc) found" "$(system_pkg_hint build-essential gcc)"
}

check_clang() { # boltffi links the JNI desktop library with clang on Linux and macOS
    [ "$(host_os)" = unsupported ] && return
    command -v clang >/dev/null && ok "clang" || bad "clang not found (boltffi uses it to link the JNI library)" "$(system_pkg_hint clang clang)"
}

python_runtime_problem() { # command -> prints why it is not wheel-build ready; 0 when ready
    local py="${1:?}" version include platinclude
    if ! command -v "$py" >/dev/null 2>&1; then
        echo "$py not found"
        return 1
    fi
    version="$("$py" --version 2>&1)"
    "$py" -c 'import sys; raise SystemExit(0 if sys.version_info[:3] >= tuple(int(part) for part in sys.argv[1].split(".")) else 1)' \
        "$PYTHON_MIN_VERSION" >/dev/null 2>&1 || {
        echo "$version is older than $PYTHON_MIN_VERSION"
        return 1
    }
    "$py" -c 'import venv, ensurepip' >/dev/null 2>&1 || {
        echo "$version has no working venv/ensurepip module"
        return 1
    }
    "$py" -m pip --version >/dev/null 2>&1 || {
        echo "$version has no working pip module"
        return 1
    }
    include="$("$py" -c 'import sysconfig; print(sysconfig.get_paths().get("include", ""))' 2>/dev/null || true)"
    platinclude="$("$py" -c 'import sysconfig; print(sysconfig.get_paths().get("platinclude", ""))' 2>/dev/null || true)"
    if { [ -n "$include" ] && [ -f "$include/Python.h" ]; } ||
        { [ -n "$platinclude" ] && [ -f "$platinclude/Python.h" ]; }; then
        :
    else
        echo "$version has no Python.h (development headers missing)"
        return 1
    fi
    "$py" -c 'import ssl, zlib' >/dev/null 2>&1 || {
        echo "$version has no working ssl/zlib modules (pip cannot download or unpack wheels)"
        return 1
    }
    return 0
}

check_python_runtime() {
    local py="${PYTHON:-$PYTHON_BIN}" problem
    problem="$(python_runtime_problem "$py" 2>&1 || true)"
    if [ -z "$problem" ]; then
        ok "$("$py" --version 2>&1) with venv, pip, headers, ssl and zlib"
    else
        bad "Python is not wheel-build ready: $problem" "make bind-python"
    fi
}

check_python() {
    echo "python"
    check_python_runtime
    check_c_compiler
}

python_command() { # the interpreter the Python targets must use (never the system one)
    if [ -n "${PYTHON:-}" ]; then
        if [ -x "$PYTHON" ] || command -v "$PYTHON" >/dev/null 2>&1; then
            printf '%s\n' "$PYTHON"
            return 0
        fi
        return 1
    fi
    [ -x "$PYTHON_BIN" ] || return 1
    printf '%s\n' "$PYTHON_BIN"
}

macos_openssl_prefix() { # Homebrew's OpenSSL 3, which macOS does not ship
    if [ "$(host_os)" = macos ] && command -v brew >/dev/null 2>&1; then
        brew --prefix openssl@3 2>/dev/null || true
    fi
}

python_build_hint() { # dependency -> distro-neutral install examples
    case "$1" in
        zlib)
            cat <<'EOF'
  Debian/Ubuntu: sudo apt install zlib1g-dev
  Fedora/RHEL:   sudo dnf install zlib-devel
  Arch:          sudo pacman -S zlib
  openSUSE:      sudo zypper install zlib-devel
EOF
            ;;
        openssl)
            cat <<'EOF'
  Debian/Ubuntu: sudo apt install libssl-dev
  Fedora/RHEL:   sudo dnf install openssl-devel
  Arch:          sudo pacman -S openssl
  openSUSE:      sudo zypper install libopenssl-devel
  macOS:         brew install openssl@3
EOF
            ;;
        *)
            cat <<'EOF'
  Debian/Ubuntu: sudo apt install build-essential curl
  Fedora/RHEL:   sudo dnf install gcc make curl
  Arch:          sudo pacman -S base-devel curl
  openSUSE:      sudo zypper install gcc make curl
EOF
            ;;
    esac
}

require_python_build_deps() {
    local tool missing=0 openssl_prefix ssl_cflags="" ssl_libs="-lssl -lcrypto"
    for tool in cc make curl tar; do
        if ! command -v "$tool" >/dev/null 2>&1; then
            echo "python: missing build tool: $tool" >&2
            missing=1
        fi
    done
    if command -v cc >/dev/null 2>&1; then
        if ! printf '#include <zlib.h>\nint main(void){return 0;}\n' | cc -x c - -o /dev/null >/dev/null 2>&1; then
            echo "python: missing zlib development headers" >&2
            python_build_hint zlib >&2
            missing=1
        fi
        openssl_prefix="$(macos_openssl_prefix)"
        if [ -n "$openssl_prefix" ]; then
            ssl_cflags="-I$openssl_prefix/include"
            ssl_libs="-L$openssl_prefix/lib -lssl -lcrypto"
        fi
        # shellcheck disable=SC2086 # flags must word-split
        if ! printf '#include <openssl/ssl.h>\nint main(void){return 0;}\n' |
            cc -x c $ssl_cflags - $ssl_libs -o /dev/null >/dev/null 2>&1; then
            echo "python: missing OpenSSL development headers or libraries" >&2
            python_build_hint openssl >&2
            missing=1
        fi
    fi
    if [ "$missing" -ne 0 ]; then
        echo "python: install the packages above, then re-run 'make bind-python'" >&2
        exit 1
    fi
}

check_jdk() {
    if ! command -v javac >/dev/null; then
        bad "javac not found (a JRE is not enough; a full JDK is needed)" "make bind-java"
    elif [ -z "${JAVA_HOME:-}" ] || [ ! -f "$JAVA_HOME/include/jni.h" ]; then
        bad "JAVA_HOME is not a JDK (no include/jni.h): ${JAVA_HOME:-unset}" "make bind-java, or point JAVA_HOME at a JDK"
    else
        ok "JDK: $(javac -version 2>&1) (JAVA_HOME=$JAVA_HOME)"
    fi
}

check_java() {
    echo "java"
    check_jdk
    check_clang
}

check_kotlin() {
    echo "kotlin"
    check_jdk
    check_clang
    command -v kotlinc >/dev/null && ok "kotlinc" || bad "kotlinc not found" "make bind-kotlin"
    command -v rustup >/dev/null && ok "rustup" || bad "rustup not found (needed to add the Android Rust target)" "install Rust from https://rustup.rs"
    if [ -n "${ANDROID_NDK_HOME:-}" ] && [ -d "$ANDROID_NDK_HOME" ]; then
        ok "Android NDK: $ANDROID_NDK_HOME"
    else
        bad "Android NDK not found (ANDROID_NDK_HOME=${ANDROID_NDK_HOME:-unset})" "make bind-kotlin, or set ANDROID_NDK_HOME"
    fi
}

check_dart() {
    echo "dart"
    if ! command -v dart >/dev/null; then
        bad "dart not found" "make bind-dart"
        return
    fi
    local v
    v="$(dart --version 2>&1 | sed -n 's/.*version: \([0-9.]*\).*/\1/p')"
    if version_ge "$v" "$DART_MIN_VERSION"; then
        ok "Dart $v"
    else
        bad "Dart $v is older than $DART_MIN_VERSION" "make bind-dart"
    fi
}

check_swift() {
    echo "swift"
    if [ "$(host_os)" != macos ]; then
        bad "the Swift test packs an Apple xcframework, so it needs macOS (CI runs it on macos-latest)" \
            "use a Mac, or run 'make test-bind-all' (Swift is skipped on Linux)"
        return
    fi
    command -v xcodebuild >/dev/null && ok "xcodebuild" || bad "xcodebuild not found" "install Xcode from the App Store, then: sudo xcode-select -s /Applications/Xcode.app"
    command -v swift >/dev/null && ok "$(swift --version 2>&1 | head -1)" || bad "swift not found" "install Xcode"
}

passes() { # quietly run check_<name>; succeed only if it reported no problems
    (failures=0; "check_$1" >/dev/null; [ "$failures" -eq 0 ])
}

doctor() {
    if [ $# -eq 0 ]; then # default: everything this machine can test (Swift needs macOS)
        set -- cli python java kotlin dart
        [ "$(host_os)" = macos ] && set -- "$@" swift
    fi
    local what
    for what in "$@"; do "check_$what"; done
    if [ "$failures" -gt 0 ]; then
        echo "doctor: $failures problem(s) found" >&2
        exit 1
    fi
    echo "doctor: all good"
}

# --- installs -------------------------------------------------------------------

install_boltffi() {
    local want
    want="$(boltffi_version)"
    if command -v boltffi >/dev/null && [ "$(boltffi --version | awk '{print $2}')" = "$want" ]; then
        echo "boltffi $want already installed"
        return
    fi
    cargo install boltffi_cli --version "$want" --locked
}

python_build_failed() { # log tmpdir
    echo "python: build failed; last 40 lines of $1:" >&2
    tail -n 40 "$1" >&2 || true
    rm -rf "$2"
    exit 1
}

install_python() {
    if [ -x "$PYTHON_BIN" ] && python_runtime_problem "$PYTHON_BIN" >/dev/null 2>&1; then
        export PYTHON="$PYTHON_BIN"
        export PATH="$PYTHON_PREFIX/bin:$PATH"
        echo "python: already available ($("$PYTHON_BIN" --version 2>&1) at $PYTHON_BIN)"
        return
    fi
    require_supported_host
    require_python_build_deps

    local openssl_prefix tmp src tarball log actual problem
    openssl_prefix="$(macos_openssl_prefix)"
    if [ "$(host_os)" = macos ] && [ -z "$openssl_prefix" ]; then
        echo "python: warning: OpenSSL 3 was not found via Homebrew; install it with" >&2
        echo "        'brew install openssl@3' or install Python $PYTHON_VERSION from python.org" >&2
    fi

    tmp="$(mktemp -d)"
    tarball="$tmp/Python-$PYTHON_VERSION.tgz"
    log="$TC_DIR/python-$PYTHON_VERSION-build.log"
    : > "$log"

    if ! download "$PYTHON_SOURCE_URL" "$tarball"; then
        rm -rf "$tmp"
        exit 1
    fi
    if ! actual="$(sha256_file "$tarball")"; then
        rm -rf "$tmp"
        exit 1
    fi
    if [ "$actual" != "$PYTHON_SHA256" ]; then
        echo "python: SHA-256 mismatch for $PYTHON_SOURCE_URL" >&2
        echo "  expected: $PYTHON_SHA256" >&2
        echo "  actual:   $actual" >&2
        rm -rf "$tmp"
        exit 1
    fi

    echo "  building CPython $PYTHON_VERSION (log: .toolchains/python-$PYTHON_VERSION-build.log)"
    if ! tar -xzf "$tarball" -C "$tmp"; then
        rm -rf "$tmp"
        exit 1
    fi
    src="$tmp/Python-$PYTHON_VERSION"

    local -a configure_args=(
        "--prefix=$PYTHON_PREFIX"
        --with-ensurepip=install
        --disable-test-modules
        --with-system-libmpdec=no
    )
    if [ -n "$openssl_prefix" ]; then
        configure_args+=("--with-openssl=$openssl_prefix")
    fi

    if ! (cd "$src" && ./configure "${configure_args[@]}") >>"$log" 2>&1; then
        python_build_failed "$log" "$tmp"
    fi
    if ! (cd "$src" && make -j"$(cpu_count)") >>"$log" 2>&1; then
        python_build_failed "$log" "$tmp"
    fi
    rm -rf "$PYTHON_PREFIX"
    if ! (cd "$src" && make install) >>"$log" 2>&1; then
        python_build_failed "$log" "$tmp"
    fi
    rm -rf "$tmp"

    if ! problem="$(python_runtime_problem "$PYTHON_BIN")"; then
        echo "python: built Python $PYTHON_VERSION but it is still not wheel-build ready: $problem" >&2
        echo "        see $log" >&2
        exit 1
    fi
    export PYTHON="$PYTHON_BIN"
    export PATH="$PYTHON_PREFIX/bin:$PATH"
    echo "python: installed $("$PYTHON_BIN" --version 2>&1) into .toolchains/python-$PYTHON_VERSION"
}

install_jdk() {
    if passes jdk; then
        echo "jdk: already available (JAVA_HOME=$JAVA_HOME)"
        return
    fi
    require_supported_host
    local os arch tmp
    os="$(host_os)"; [ "$os" = macos ] && os=mac
    arch="$(host_arch)"; arch="${arch/x86_64/x64}"; arch="${arch/arm64/aarch64}"
    tmp="$(mktemp -d)"
    download "https://api.adoptium.net/v3/binary/latest/$JDK_MAJOR/ga/$os/$arch/jdk/hotspot/normal/eclipse" "$tmp/jdk.tar.gz"
    tar -xzf "$tmp/jdk.tar.gz" -C "$tmp"
    rm -rf "$TC_DIR/jdk"
    local home
    home="$(find "$tmp" -maxdepth 4 -type f -path '*/bin/javac' | head -1)"
    mv "$(dirname "$(dirname "$home")")" "$TC_DIR/jdk"
    rm -rf "$tmp"
    echo "jdk: installed Temurin $JDK_MAJOR into .toolchains/jdk"
}

install_java() {
    install_jdk
}

install_kotlin() {
    install_jdk
    require_supported_host
    if command -v kotlinc >/dev/null; then
        echo "kotlinc: already available"
    else
        local tmp
        tmp="$(mktemp -d)"
        download "https://github.com/JetBrains/kotlin/releases/download/v$KOTLIN_VERSION/kotlin-compiler-$KOTLIN_VERSION.zip" "$tmp/kotlinc.zip"
        rm -rf "$TC_DIR/kotlinc"
        unzip -q "$tmp/kotlinc.zip" -d "$TC_DIR"
        rm -rf "$tmp"
        echo "kotlinc: installed $KOTLIN_VERSION into .toolchains/kotlinc"
    fi
    if [ -n "${ANDROID_NDK_HOME:-}" ] && [ -d "$ANDROID_NDK_HOME" ]; then
        echo "ndk: already available ($ANDROID_NDK_HOME)"
    else
        install_ndk
    fi
}

install_ndk() {
    local tmp
    tmp="$(mktemp -d)"
    rm -rf "$TC_DIR/ndk"
    if [ "$(host_os)" = linux ]; then
        download "https://dl.google.com/android/repository/android-ndk-$NDK_VERSION-linux.zip" "$tmp/ndk.zip"
        unzip -q "$tmp/ndk.zip" -d "$tmp"
        mv "$tmp/android-ndk-$NDK_VERSION" "$TC_DIR/ndk"
    else
        download "https://dl.google.com/android/repository/android-ndk-$NDK_VERSION-darwin.dmg" "$tmp/ndk.dmg"
        hdiutil attach -quiet -nobrowse -mountpoint "$tmp/mnt" "$tmp/ndk.dmg"
        cp -R "$(find "$tmp/mnt" -maxdepth 4 -type d -name NDK | head -1)" "$TC_DIR/ndk"
        hdiutil detach -quiet "$tmp/mnt"
    fi
    rm -rf "$tmp"
    export ANDROID_NDK_HOME="$TC_DIR/ndk"
    echo "ndk: installed $NDK_VERSION into .toolchains/ndk"
}

install_dart() {
    if passes dart; then
        echo "dart: already available"
        return
    fi
    require_supported_host
    local os arch tmp
    os="$(host_os)"
    arch="$(host_arch)"; arch="${arch/x86_64/x64}"
    tmp="$(mktemp -d)"
    download "https://storage.googleapis.com/dart-archive/channels/stable/release/$DART_VERSION/sdk/dartsdk-$os-$arch-release.zip" "$tmp/dart.zip"
    rm -rf "$TC_DIR/dart-sdk"
    unzip -q "$tmp/dart.zip" -d "$TC_DIR"
    rm -rf "$tmp"
    echo "dart: installed $DART_VERSION into .toolchains/dart-sdk"
}

install_swift() {
    if [ "$(host_os)" != macos ]; then
        echo "swift: skipped, the Swift test needs macOS with Xcode (CI runs it on macos-latest)"
        return
    fi
    if passes swift; then
        echo "swift: already available"
    else
        echo "swift: Xcode can't be installed from the command line. Install it from the App Store, then run: sudo xcode-select -s /Applications/Xcode.app" >&2
        exit 1
    fi
}

install() {
    local lang="${1:?usage: toolchains.sh install <lang>|all}"
    if [ "$lang" = all ]; then
        for lang in python java kotlin dart swift; do "install_$lang"; done
    else
        "install_$lang"
    fi
}

# --- main -----------------------------------------------------------------------

setup_env
cmd="${1:?usage: toolchains.sh install-boltffi | install <lang> | doctor [cli] [<lang>...] | run <cmd...>}"
shift
case "$cmd" in
    install-boltffi) install_boltffi ;;
    install) install "$@" ;;
    doctor) doctor "$@" ;;
    run)
        if [ -z "${PYTHON:-}" ] && selected_python="$(python_command)"; then
            export PYTHON="$selected_python"
        fi
        exec "$@"
        ;;
    *) echo "unknown command: $cmd" >&2; exit 2 ;;
esac

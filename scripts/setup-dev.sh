#!/usr/bin/env bash
# setup-dev.sh
# Installs build-essential equivalents, rustup (nightly), llvm-tools-preview,
# cargo bootimage, qemu, llvm/clang, and the x86_64-linux-gnu cross compiler.
#
# Works with:
#   - apt (Debian/Ubuntu)
#   - pacman (Arch)
#   - dnf (Fedora/RHEL)
#   - brew (macOS/Homebrew)
#
# for some reason i really liked this language, its different. but i hate powershell even
# though its similar but higher level

set -euo pipefail

CI_MODE="${CI:-false}"
ASSUME_YES=0

usage() {
  cat <<'EOF'
Usage: setup-dev.sh [--yes|-y] [--non-interactive]

  --yes, -y, --non-interactive  Skip confirmation prompts.
  --help, -h                    Show help.
EOF
}

for arg in "${@:-}"; do
  case "$arg" in
    -y|--yes|--non-interactive)
      ASSUME_YES=1
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $arg"
      usage
      exit 2
      ;;
  esac
done

echo "=== dev-setup: starting ==="

if [ "$(id -u)" -eq 0 ]; then
  if [[ "$CI_MODE" == "true" ]]; then
    SUDO_PREFIX=()
  else
    echo "Do NOT run this script as root. Run it as your normal user; it will use sudo internally for package manager operations."
    exit 1
  fi
else
  SUDO_PREFIX=(sudo)
fi

run_as_root() {
  if [ "${#SUDO_PREFIX[@]}" -eq 0 ]; then
    "$@"
  else
    "${SUDO_PREFIX[@]}" "$@"
  fi
}

confirm() {
  local prompt="$1"

  if [[ "$ASSUME_YES" -eq 1 || "$CI_MODE" == "true" ]]; then
    echo "$prompt [auto-yes]"
    return 0
  fi

  echo
  echo "WARNING: This script will install and update multiple development packages and tools on your system."
  echo "It will use your system's package manager (apt, pacman, dnf, or brew) and may make significant changes."
  echo
  read -r -p "$prompt [Y/n] " resp
  resp=${resp:-Y}
  [[ "$resp" =~ ^[Yy]$ ]]
}

install_on_apt() {
  echo "-- Detected apt (Debian/Ubuntu). Installing build-essential, llvm, qemu, python..."
  run_as_root apt update
  run_as_root apt install -y build-essential curl git ca-certificates uuid-dev nasm acpica-tools ovmf dosfstools parted \
      qemu-system-x86 qemu-utils clang python3 xorriso grub-pc-bin python3-pyelftools \
      gcc-x86-64-linux-gnu binutils-x86-64-linux-gnu

  tmp_llvm="$(mktemp)"
  curl -fsSL https://apt.llvm.org/llvm.sh -o "$tmp_llvm"
  run_as_root bash "$tmp_llvm"
  rm -f "$tmp_llvm"

  echo "-- apt installs finished"
}

install_on_pacman() {
  echo "-- Detected pacman (Arch). Installing base-devel group, llvm, qemu, python..."
  run_as_root pacman -Sy --noconfirm

  run_as_root pacman -S --needed --noconfirm \
    base-devel \
    qemu-desktop \
    llvm \
    clang \
    curl \
    git \
    python \
    ovmf \
    nasm \
    acpica \
    dosfstools \
    parted \
    grub \
    xorriso \
    python-pyelftools

  echo "-- pacman installs finished"
}

install_on_dnf() {
  echo "-- Detected dnf (Fedora/RHEL). Installing Development Tools + qemu + llvm..."
  if run_as_root dnf -y groupinstall "Development Tools" >/dev/null 2>&1; then
    echo "-- dnf groupinstall completed"
  else
    echo "-- dnf groupinstall failed/unsupported: falling back to core packages"
    run_as_root dnf -y install make automake gcc gcc-c++ kernel-devel
  fi

  run_as_root dnf -y install \
    qemu-kvm \
    qemu-img \
    qemu-system-x86 \
    llvm \
    clang \
    curl \
    git \
    python3 \
    libuuid-devel \
    nasm \
    acpica-tools \
    edk2-ovmf \
    dosfstools \
    parted \
    grub2-tools \
    xorriso \
    gcc-x86_64-linux-gnu \
    python3-pyelftools || \
  run_as_root dnf -y install \
    qemu \
    qemu-img \
    llvm \
    clang \
    curl \
    git \
    python3 \
    libuuid-devel \
    nasm \
    acpica-tools \
    edk2-ovmf \
    dosfstools \
    parted \
    grub2-tools \
    xorriso \
    gcc-x86_64-linux-gnu \
    python3-pyelftools

  echo "-- dnf installs finished"
}

install_on_brew() {
  echo "-- Detected macOS/Homebrew. Ensuring Xcode Command Line Tools and Homebrew..."

  if ! xcode-select -p >/dev/null 2>&1; then
    if [[ "$CI_MODE" == "true" ]]; then
      echo "Xcode Command Line Tools are missing. Install them before running this script on macOS."
      exit 1
    fi

    echo "Installing Xcode Command Line Tools..."
    xcode-select --install || true
    echo "Complete the installation and rerun this script."
  else
    echo "Xcode Command Line Tools already present"
  fi

  if ! command -v brew >/dev/null 2>&1; then
    if [[ "$CI_MODE" == "true" ]]; then
      echo "Homebrew is not available."
      exit 1
    fi

    echo "Homebrew not found — attempting to install..."
    /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

    if [[ -x /opt/homebrew/bin/brew ]]; then
      eval "$(/opt/homebrew/bin/brew shellenv)"
    elif [[ -x /usr/local/bin/brew ]]; then
      eval "$(/usr/local/bin/brew shellenv)"
    fi
  fi

  if ! command -v brew >/dev/null 2>&1; then
    echo "ERROR: Homebrew is unavailable."
    exit 1
  fi

  echo "-- Updating Homebrew..."
  brew update || true

  echo "-- Installing macOS build dependencies..."
  brew install \
    curl \
    git \
    ca-certificates \
    ossp-uuid \
    nasm \
    acpica \
    ovmf \
    dosfstools \
    parted \
    qemu \
    llvm \
    python \
    xorriso \
    x86_64-elf-grub

  echo "-- Installing Python build dependencies..."
  python3 -m pip install --user --break-system-packages pyelftools || true

  echo "-- Installing x86_64 Linux cross compiler..."
  brew tap messense/macos-cross-toolchains
  brew trust messense/macos-cross-toolchains
  brew install x86_64-unknown-linux-gnu

  local brew_prefix
  brew_prefix="$(brew --prefix)"
  export PATH="$brew_prefix/bin:$PATH"
  export PATH="$HOME/Library/Python/3.*/bin:$PATH"

  if [[ -n "${GITHUB_PATH:-}" ]]; then
    echo "$brew_prefix/bin" >> "$GITHUB_PATH"
    echo "$HOME/Library/Python/3.*/bin" >> "$GITHUB_PATH" || true
  fi

  if ! command -v x86_64-linux-gnu-gcc >/dev/null 2>&1; then
    echo "ERROR: x86_64-linux-gnu-gcc was not found after installation."
    echo "Expected it in: $brew_prefix/bin"
    exit 1
  fi

  echo "-- Cross compiler:"
  x86_64-linux-gnu-gcc --version | head -n 1

  echo "-- Compiler target:"
  x86_64-linux-gnu-gcc -dumpmachine

  echo "-- brew installs finished"
}

if ! confirm "Continue with installation?"; then
  echo "Aborted by user."
  exit 0
fi

# detect package manager / OS
if command -v apt >/dev/null 2>&1; then
  install_on_apt
elif command -v pacman >/dev/null 2>&1; then
  install_on_pacman
elif command -v dnf >/dev/null 2>&1; then
  install_on_dnf
elif [[ "${OSTYPE:-}" == darwin* ]] || command -v brew >/dev/null 2>&1; then
  install_on_brew
else
  echo "Unsupported OS / package manager. This script handles apt, pacman, dnf, and Homebrew."
  exit 2
fi

echo
echo "-- Installing rustup (non-interactive) and setting default toolchain to nightly..."

if ! command -v curl >/dev/null 2>&1; then
  echo "curl not installed — attempting to install curl first..."

  if command -v apt >/dev/null 2>&1; then
    run_as_root apt install -y curl
  fi

  if command -v pacman >/dev/null 2>&1; then
    run_as_root pacman -S --noconfirm --needed curl
  fi

  if command -v dnf >/dev/null 2>&1; then
    run_as_root dnf install -y curl
  fi
fi

curl --proto '=https' -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain nightly

if [ -f "$HOME/.cargo/env" ]; then
  # shellcheck disable=SC1090
  source "$HOME/.cargo/env"
fi

if command -v rustup >/dev/null 2>&1; then
  rustup install nightly || true
  rustup default nightly || true
else
  echo "rustup not found after install — check the rustup installer output above."
fi

echo "-- Adding llvm-tools-preview (or fallback to llvm-tools) to nightly toolchain..."

if command -v rustup >/dev/null 2>&1; then
  if ! rustup component add llvm-tools-preview rust-src --toolchain nightly >/dev/null 2>&1; then
    echo "llvm-tools-preview not available; trying llvm-tools..."

    if ! rustup component add llvm-tools rust-src --toolchain nightly >/dev/null 2>&1; then
      echo "Couldn't add an llvm-tools rustup component (it may not be available for this platform/toolchain)."
      echo "You can still use system llvm/clang or install llvm tools separately."
    else
      echo "Added 'llvm-tools' & 'rust-src' components."
    fi
  else
    echo "Added 'llvm-tools-preview' & 'rust-src' components."
  fi
fi

echo "-- Installing cargo subcommand: bootimage"

if command -v cargo >/dev/null 2>&1; then
  cargo install bootimage || echo "cargo install bootimage failed; try 'cargo install bootimage' manually"
else
  echo "cargo not found — rustup may not have finished; make sure ~/.cargo/bin is on your PATH and re-run 'cargo install bootimage'"
fi

echo
echo "=== Setup summary ==="

printf "Host: %s\n" "$(uname -a 2>/dev/null || true)"

ver_if() {
  local cmd="$1"
  local label="$2"

  if command -v "$cmd" >/dev/null 2>&1; then
    printf "%-28s: %s\n" "$label" "$("$cmd" --version 2>&1 | head -n1)"
  else
    printf "%-28s: %s\n" "$label" "not found"
  fi
}

ver_if gcc "gcc"
ver_if x86_64-linux-gnu-gcc "cross gcc"
ver_if x86_64-linux-gnu-ld "cross ld"
ver_if clang "clang"
ver_if rustc "rustc"
ver_if cargo "cargo"

if command -v qemu-system-x86_64 >/dev/null 2>&1; then
  printf "%-28s: %s\n" "qemu" "$(qemu-system-x86_64 --version 2>&1 | head -n1)"
elif command -v qemu-system-x86 >/dev/null 2>&1; then
  printf "%-28s: %s\n" "qemu" "$(qemu-system-x86 --version 2>&1 | head -n1)"
elif command -v qemu >/dev/null 2>&1; then
  printf "%-28s: %s\n" "qemu" "$(qemu --version 2>&1 | head -n1)"
else
  printf "%-28s: %s\n" "qemu" "not found"
fi

if command -v rustup >/dev/null 2>&1; then
  echo "rustup toolchains installed:"
  rustup toolchain list || true

  echo "Installed rustup components for nightly:"
  rustup component list --toolchain nightly --installed || true
fi

echo
echo "Local PATH additions attempted:"
echo "  - rustup/cargo: ~/.cargo/bin (sourced via ~/.cargo/env if present)"
echo
echo "If any step failed, re-run the failing command manually and check the output above."
echo
echo "Please run: source \"$HOME/.cargo/env\""
echo "=== dev-setup: finished ==="
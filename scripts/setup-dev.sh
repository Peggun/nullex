#!/usr/bin/env bash
# setup-dev.sh
# Installs build-essential equivalents, rustup (nightly), llvm-tools-preview, cargo bootimage, qemu, llvm/clang.
# Works with: apt (Debian/Ubuntu), pacman (Arch), dnf (Fedora/RHEL), brew (macOS/Homebrew).
#
# for some reason i really liked this language, its different. but i hate powershell even 
# though its similar but higher level

set -euo pipefail

echo "=== dev-setup: starting ==="

SUDO_PREFIX="sudo"
if [ "$(id -u)" -eq 0 ]; then
  echo "Do NOT run this script as root. Run it as your normal user; it will use sudo internally for package manager operations."
  exit 1
fi

install_on_apt() {
  echo "-- Detected apt (Debian/Ubuntu). Installing build-essential, llvm, qemu, python..."
  $SUDO_PREFIX apt update
  $SUDO_PREFIX apt install -y build-essential curl git ca-certificates uuid-dev nasm acpica-tools ovmf dosfstools parted \
      qemu-system-x86 qemu-utils clang python3
  $SUDO_PREFIX bash -c "$(wget -O - https://apt.llvm.org/llvm.sh)"
  echo "-- apt installs finished"
}

install_on_pacman() {
  echo "-- Detected pacman (Arch). Installing base-devel group, llvm, qemu, python..."
  $SUDO_PREFIX pacman -Sy --noconfirm
  $SUDO_PREFIX pacman -S --needed --noconfirm base-devel qemu llvm clang curl git python ovmf libuuid nasm acpica ovmf dosfstools parted
  echo "-- pacman installs finished"
}

install_on_dnf() {
  echo "-- Detected dnf (Fedora/RHEL). Installing Development Tools + qemu + llvm..."
  # try to groupinstall. fallback to core packages if groupinstall fails
  if $SUDO_PREFIX dnf -y groupinstall "Development Tools" "Development Libraries" >/dev/null 2>&1; then
    echo "-- dnf groupinstall completed"
  else
    echo "-- dnf groupinstall failed/unsupported: falling back to core packages"
    $SUDO_PREFIX dnf -y install make automake gcc gcc-c++ kernel-devel 
  fi

  # install qemu/kvm and llvm/clang and python
  $SUDO_PREFIX dnf -y install qemu-kvm qemu-img qemu-system-x86 llvm clang curl git || \
    $SUDO_PREFIX dnf -y install qemu qemu-img llvm clang curl git python3 libuuid-devel nasm acpica-tools edk2-ovmf dosfstools parted
  echo "-- dnf installs finished"
}

install_on_brew() {
  echo "-- Detected macOS/Homebrew. Ensuring Xcode Command Line Tools and Homebrew..."
  # try to install Xcode Command Line Tools (may prompt GUI installer)
  if ! xcode-select -p >/dev/null 2>&1; then
    echo "Installing Xcode Command Line Tools (this may pop up a GUI prompt)..."
    xcode-select --install || true
    echo "If a GUI prompt appeared, complete that install and re-run this script if needed."
  else
    echo "Xcode Command Line Tools already present"
  fi

  # install Homebrew if missing (best-effort; user interaction may be required)
  if ! command -v brew >/dev/null 2>&1; then
    echo "Homebrew not found — attempting to install Homebrew (may require interaction)"
    /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)" || true
    eval "$(/opt/homebrew/bin/brew shellenv 2>/dev/null || true)" || true
  fi

  if command -v brew >/dev/null 2>&1; then
    brew update || true
    brew install qemu llvm curl git || true
  else
    echo "Homebrew still not available. Please install Homebrew manually: https://brew.sh/"
    exit 1
  fi
  echo "-- brew installs finished"
}

# detect package manager / OS
if command -v apt >/dev/null 2>&1; then
  install_on_apt
elif command -v pacman >/dev/null 2>&1; then
  install_on_pacman
elif command -v dnf >/dev/null 2>&1; then
  install_on_dnf
elif [[ "$OSTYPE" == darwin* ]] || command -v brew >/dev/null 2>&1; then
  install_on_brew
else
  echo "Unsupported OS / package manager. This script handles apt, pacman, dnf, and Homebrew."
  exit 2
fi

echo
echo "WARNING: This script will install and update multiple development packages and tools on your system."
echo "It will use your system's package manager (apt, pacman, dnf, or brew) and may make significant changes."
echo
read -p "Continue with installation? [Y/n] " resp
resp=${resp:-Y}
if [[ ! "$resp" =~ ^[Yy]$ ]]; then
  echo "Aborted by user."
  exit 0
fi

echo
read -p "Are you absolutely sure you want to proceed? [Y/n] " sure
sure=${sure:-Y}
if [[ ! "$sure" =~ ^[Yy]$ ]]; then
  echo "Aborted by user."
  exit 0
fi
echo

# --- install rustup non-interactively and default to nightly ---
echo "-- Installing rustup (non-interactive) and setting default toolchain to nightly..."
if ! command -v curl >/dev/null 2>&1; then
  echo "curl not installed — attempting to install curl first..."
  if command -v apt >/dev/null 2>&1; then $SUDO_PREFIX apt install -y curl; fi
  if command -v pacman >/dev/null 2>&1; then $SUDO_PREFIX pacman -S --noconfirm --needed curl; fi
  if command -v dnf >/dev/null 2>&1; then $SUDO_PREFIX dnf install -y curl; fi
fi

# use the official rustup install script -y makes it non-interactive
curl --proto '=https' -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain nightly

# ensure current shell can use cargo/rustup immediately
if [ -f "$HOME/.cargo/env" ]; then
  # shellcheck disable=SC1090
  source "$HOME/.cargo/env"
fi

# make sure nightly is installed and set as default
if command -v rustup >/dev/null 2>&1; then
  rustup install nightly || true
  rustup default nightly || true
else
  echo "rustup not found after install — check the rustup installer output above."
fi

# try to add the llvm-tools component (name varies across time/platforms)
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

# --- install cargo bootimage ---
echo "-- Installing cargo subcommand: bootimage"
if command -v cargo >/dev/null 2>&1; then
  cargo install bootimage || echo "cargo install bootimage failed; try 'cargo install bootimage' manually"
else
  echo "cargo not found — rustup may not have finished; make sure ~/.cargo/bin is on your PATH and re-run 'cargo install bootimage'"
fi

# --- final: quick summary of installed versions ---
echo
echo "=== Setup summary ==="
printf "Host: %s\n" "$(uname -a 2>/dev/null || true)"

# helper to print version if command exists
ver_if() {
  local cmd="$1"
  local label="$2"
  if command -v "$cmd" >/dev/null 2>&1; then
    printf "%-22s: %s\n" "$label" "$($cmd --version 2>&1 | head -n1)"
  else
    printf "%-22s: %s\n" "$label" "not found"
  fi
}

ver_if gcc "gcc"
ver_if clang "clang"
ver_if rustc "rustc"
ver_if cargo "cargo"
# qemu variant: try several common names
if command -v qemu-system-x86_64 >/dev/null 2>&1; then
  printf "%-22s: %s\n" "qemu" "$(qemu-system-x86_64 --version 2>&1 | head -n1)"
elif command -v qemu-system-x86 >/dev/null 2>&1; then
  printf "%-22s: %s\n" "qemu" "$(qemu-system-x86 --version 2>&1 | head -n1)"
elif command -v qemu >/dev/null 2>&1; then
  printf "%-22s: %s\n" "qemu" "$(qemu --version 2>&1 | head -n1)"
else
  printf "%-22s: %s\n" "qemu" "not found"
fi

# llvm-tools installed via rustup?
if command -v rustup >/dev/null 2>&1; then
  echo "rustup toolchains installed: "
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
echo "Please run source "$HOME/.cargo/env" as this script doesnt like it"
echo "=== dev-setup: finished ==="

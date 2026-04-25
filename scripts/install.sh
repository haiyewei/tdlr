#!/usr/bin/env bash
set -euo pipefail

BIN_NAME="tdlr"
DEFAULT_SOURCE_MODE="${TDLR_INSTALL_SOURCE:-auto}"
REMOTE_REPO_OWNER="${TDLR_REMOTE_REPO_OWNER:-haiyewei}"
REMOTE_REPO_NAME="${TDLR_REMOTE_REPO_NAME:-tdlr}"
REMOTE_VERSION="${TDLR_INSTALL_VERSION:-latest}"
REMOTE_TARGET="${TDLR_INSTALL_TARGET:-auto}"
INSTALL_DIR="${TDLR_INSTALL_DIR:-}"
PROXY_PREFIX=""
SCRIPT_PATH="${BASH_SOURCE[0]:-}"
SCRIPT_DIR=""
REPO_ROOT=""

if [[ -n "${SCRIPT_PATH}" ]]; then
  SCRIPT_DIR="$(cd -- "$(dirname -- "${SCRIPT_PATH}")" && pwd)"
  REPO_ROOT="$(cd -- "${SCRIPT_DIR}/.." 2>/dev/null && pwd || true)"
fi

while [[ $# -gt 0 ]]; do
  case "$1" in
    --source)
      DEFAULT_SOURCE_MODE="$2"
      shift 2
      ;;
    --version)
      REMOTE_VERSION="$2"
      shift 2
      ;;
    --target)
      REMOTE_TARGET="$2"
      shift 2
      ;;
    --install-dir)
      INSTALL_DIR="$2"
      shift 2
      ;;
    --proxy)
      PROXY_PREFIX="https://gh-proxy.com/"
      shift
      ;;
    *)
      printf '[tdlr] unknown flag: %s\n' "$1" >&2
      exit 1
      ;;
  esac
done

default_install_dir() {
  printf '%s\n' "${HOME}/.local/bin"
}

absolute_file_path() {
  local path="$1"
  local dir

  dir="$(cd -- "$(dirname -- "${path}")" && pwd)"
  printf '%s/%s\n' "${dir}" "$(basename -- "${path}")"
}

resolve_profile_file() {
  if [[ -n "${TDLR_PROFILE_FILE:-}" ]]; then
    printf '%s\n' "${TDLR_PROFILE_FILE}"
    return 0
  fi

  case "${SHELL##*/}" in
    bash)
      printf '%s\n' "${HOME}/.bashrc"
      ;;
    zsh)
      printf '%s\n' "${HOME}/.zshrc"
      ;;
    *)
      printf '%s\n' "${HOME}/.profile"
      ;;
  esac
}

is_sourced() {
  [[ "${BASH_SOURCE[0]}" != "${0}" ]]
}

persist_path_entry() {
  local install_dir="$1"
  local profile_file
  local profile_dir
  local path_line

  profile_file="$(resolve_profile_file)"
  profile_dir="$(dirname -- "${profile_file}")"
  path_line="export PATH=\"${install_dir}:\$PATH\""

  mkdir -p "${profile_dir}"
  touch "${profile_file}"

  if grep -Fqx "${path_line}" "${profile_file}"; then
    printf '[tdlr] PATH entry already exists in %s\n' "${profile_file}"
  else
    {
      printf '\n# tdlr installer\n'
      printf '%s\n' "${path_line}"
    } >> "${profile_file}"
    printf '[tdlr] added install directory to %s\n' "${profile_file}"
  fi

  if [[ ":${PATH}:" == *":${install_dir}:"* ]]; then
    return 0
  fi

  if is_sourced; then
    export PATH="${install_dir}:${PATH}"
    printf '[tdlr] updated PATH in the current shell session\n'
  else
    printf '[tdlr] restart your shell or run the following command to refresh PATH now:\n'
    printf '  source "%s"\n' "${profile_file}"
  fi
}

resolve_execution_mode() {
  case "${DEFAULT_SOURCE_MODE}" in
    auto|local|remote)
      ;;
    *)
      printf '[tdlr] unsupported install source mode: %s\n' "${DEFAULT_SOURCE_MODE}" >&2
      exit 1
      ;;
  esac

  if [[ "${DEFAULT_SOURCE_MODE}" != "auto" ]]; then
    printf '%s\n' "${DEFAULT_SOURCE_MODE}"
    return 0
  fi

  if [[ -n "${SCRIPT_DIR}" && -f "${SCRIPT_DIR}/${BIN_NAME}" ]]; then
    printf 'local\n'
    return 0
  fi

  if [[ -n "${REPO_ROOT}" && -f "${REPO_ROOT}/Cargo.toml" ]]; then
    printf 'local\n'
    return 0
  fi

  if [[ -n "${REPO_ROOT}" && -f "${REPO_ROOT}/target/release/${BIN_NAME}" ]]; then
    printf 'local\n'
    return 0
  fi

  printf 'remote\n'
}

resolve_local_binary() {
  local candidates=()

  if [[ -n "${SCRIPT_DIR}" ]]; then
    candidates+=("${SCRIPT_DIR}/${BIN_NAME}")
  fi

  if [[ -n "${REPO_ROOT}" ]]; then
    candidates+=("${REPO_ROOT}/target/release/${BIN_NAME}")
    candidates+=("${REPO_ROOT}/target/debug/${BIN_NAME}")
  fi

  local candidate
  for candidate in "${candidates[@]}"; do
    if [[ -f "${candidate}" ]]; then
      printf '%s\n' "${candidate}"
      return 0
    fi
  done

  return 1
}

has_repository_workspace() {
  [[ -n "${REPO_ROOT}" && -f "${REPO_ROOT}/Cargo.toml" ]]
}

build_local_release_binary() {
  if ! command -v cargo >/dev/null 2>&1; then
    return 1
  fi

  if [[ -z "${REPO_ROOT}" || ! -f "${REPO_ROOT}/Cargo.toml" ]]; then
    return 1
  fi

  printf '[tdlr] no local binary found, building release binary with cargo --bin %s\n' "${BIN_NAME}" >&2
  (
    cd "${REPO_ROOT}"
    cargo build --release --bin "${BIN_NAME}"
  )

  if [[ -f "${REPO_ROOT}/target/release/${BIN_NAME}" ]]; then
    printf '%s\n' "${REPO_ROOT}/target/release/${BIN_NAME}"
    return 0
  fi

  return 1
}

remote_asset_name() {
  local target
  target="$(resolve_remote_target)"

  case "${target}" in
    x86_64-unknown-linux-gnu)
      printf 'tdlr-x86_64-unknown-linux-gnu.tar.gz\n'
      ;;
    aarch64-unknown-linux-gnu)
      printf 'tdlr-aarch64-unknown-linux-gnu.tar.gz\n'
      ;;
    x86_64-unknown-linux-musl)
      printf 'tdlr-x86_64-unknown-linux-musl.tar.gz\n'
      ;;
    aarch64-unknown-linux-musl)
      printf 'tdlr-aarch64-unknown-linux-musl.tar.gz\n'
      ;;
    x86_64-apple-darwin)
      printf 'tdlr-x86_64-apple-darwin.tar.gz\n'
      ;;
    aarch64-apple-darwin)
      printf 'tdlr-aarch64-apple-darwin.tar.gz\n'
      ;;
    *)
      printf '[tdlr] unsupported remote target: %s\n' "${target}" >&2
      exit 1
      ;;
  esac
}

normalize_remote_target() {
  case "$1" in
    auto|x86_64-unknown-linux-gnu|aarch64-unknown-linux-gnu|x86_64-unknown-linux-musl|aarch64-unknown-linux-musl|x86_64-apple-darwin|aarch64-apple-darwin)
      printf '%s\n' "$1"
      ;;
    *)
      printf '[tdlr] unsupported remote target selector: %s\n' "$1" >&2
      exit 1
      ;;
  esac
}

detect_linux_libc() {
  if command -v ldd >/dev/null 2>&1; then
    local ldd_output
    ldd_output="$(ldd --version 2>&1 || true)"

    if printf '%s' "${ldd_output}" | grep -qi 'musl'; then
      printf 'musl\n'
      return 0
    fi

    if printf '%s' "${ldd_output}" | grep -qiE 'glibc|gnu libc'; then
      printf 'gnu\n'
      return 0
    fi
  fi

  if command -v getconf >/dev/null 2>&1 && getconf GNU_LIBC_VERSION >/dev/null 2>&1; then
    printf 'gnu\n'
    return 0
  fi

  printf 'gnu\n'
}

resolve_remote_target() {
  local requested_target
  requested_target="$(normalize_remote_target "${REMOTE_TARGET}")"

  if [[ "${requested_target}" != "auto" ]]; then
    printf '%s\n' "${requested_target}"
    return 0
  fi

  case "$(uname -s)" in
    Linux)
      local libc
      libc="$(detect_linux_libc)"
      case "$(uname -m)" in
        x86_64|amd64)
          printf 'x86_64-unknown-linux-%s\n' "${libc}"
          ;;
        aarch64|arm64)
          printf 'aarch64-unknown-linux-%s\n' "${libc}"
          ;;
        *)
          printf '[tdlr] unsupported architecture for remote install: %s\n' "$(uname -m)" >&2
          exit 1
          ;;
      esac
      ;;
    Darwin)
      case "$(uname -m)" in
        x86_64|amd64)
          printf 'x86_64-apple-darwin\n'
          ;;
        aarch64|arm64)
          printf 'aarch64-apple-darwin\n'
          ;;
        *)
          printf '[tdlr] unsupported architecture for remote install: %s\n' "$(uname -m)" >&2
          exit 1
          ;;
      esac
      ;;
    *)
      printf '[tdlr] unsupported operating system for remote install: %s\n' "$(uname -s)" >&2
      exit 1
      ;;
  esac
}

remote_download_url() {
  local asset_name

  asset_name="$(remote_asset_name)"

  if [[ -n "${TDLR_REMOTE_BASE_URL:-}" ]]; then
    printf '%s/%s\n' "${TDLR_REMOTE_BASE_URL%/}" "${asset_name}"
    return 0
  fi

  if [[ "${REMOTE_VERSION}" == "latest" ]]; then
    printf '%shttps://github.com/%s/%s/releases/latest/download/%s\n' \
      "${PROXY_PREFIX}" "${REMOTE_REPO_OWNER}" "${REMOTE_REPO_NAME}" "${asset_name}"
  else
    printf '%shttps://github.com/%s/%s/releases/download/%s/%s\n' \
      "${PROXY_PREFIX}" "${REMOTE_REPO_OWNER}" "${REMOTE_REPO_NAME}" "${REMOTE_VERSION}" "${asset_name}"
  fi
}

download_remote_binary() {
  local url
  local tmp_dir

  url="$(remote_download_url)"
  tmp_dir="$(mktemp -d "${TMPDIR:-/tmp}/tdlr-install.XXXXXX")"

  printf '[tdlr] downloading %s\n' "${url}" >&2

  if command -v curl >/dev/null 2>&1; then
    curl -fsSL "${url}" -o "${tmp_dir}/package.tar.gz"
  elif command -v wget >/dev/null 2>&1; then
    wget -qO "${tmp_dir}/package.tar.gz" "${url}"
  else
    printf '[tdlr] curl or wget is required for remote install.\n' >&2
    rm -rf "${tmp_dir}"
    return 1
  fi

  tar -xzf "${tmp_dir}/package.tar.gz" -C "${tmp_dir}"

  if [[ ! -f "${tmp_dir}/${BIN_NAME}" ]]; then
    printf '[tdlr] binary %s not found in the downloaded package.\n' "${BIN_NAME}" >&2
    rm -rf "${tmp_dir}"
    return 1
  fi

  printf '%s\n' "${tmp_dir}/${BIN_NAME}"
}

if [[ -z "${INSTALL_DIR}" ]]; then
  INSTALL_DIR="$(default_install_dir)"
fi

INSTALL_MODE="$(resolve_execution_mode)"
SOURCE_BINARY=""

if [[ "${INSTALL_MODE}" == "local" ]]; then
  if [[ -n "${SCRIPT_DIR}" && -f "${SCRIPT_DIR}/${BIN_NAME}" ]]; then
    SOURCE_BINARY="${SCRIPT_DIR}/${BIN_NAME}"
  elif has_repository_workspace; then
    SOURCE_BINARY="$(build_local_release_binary || true)"
    if [[ -z "${SOURCE_BINARY}" ]]; then
      SOURCE_BINARY="$(resolve_local_binary || true)"
    fi
  else
    SOURCE_BINARY="$(resolve_local_binary || true)"
  fi

  if [[ -z "${SOURCE_BINARY}" ]]; then
    printf '[tdlr] no local binary found next to the script or in target/release.\n' >&2
    printf '[tdlr] remote download is available only when this script runs in remote mode.\n' >&2
    exit 1
  fi
else
  SOURCE_BINARY="$(download_remote_binary || true)"
  if [[ -z "${SOURCE_BINARY}" ]]; then
    exit 1
  fi
fi

mkdir -p "${INSTALL_DIR}"
INSTALL_PATH="${INSTALL_DIR}/${BIN_NAME}"

if [[ "$(absolute_file_path "${SOURCE_BINARY}")" != "$(absolute_file_path "${INSTALL_PATH}")" ]]; then
  cp "${SOURCE_BINARY}" "${INSTALL_PATH}"
fi

chmod 755 "${INSTALL_PATH}"

if [[ "${INSTALL_MODE}" == "remote" ]]; then
  rm -rf "$(dirname -- "$(absolute_file_path "${SOURCE_BINARY}")")"
fi

printf '[tdlr] installed to %s\n' "${INSTALL_PATH}"
persist_path_entry "${INSTALL_DIR}"
printf '[tdlr] run "%s --help" to get started\n' "${BIN_NAME}"

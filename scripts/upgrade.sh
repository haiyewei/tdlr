#!/usr/bin/env bash
set -euo pipefail

BIN_NAME="tdlr"
DEFAULT_SOURCE_MODE="${TDLR_INSTALL_SOURCE:-remote}"
REMOTE_REPO_OWNER="${TDLR_REMOTE_REPO_OWNER:-haiyewei}"
REMOTE_REPO_NAME="${TDLR_REMOTE_REPO_NAME:-tdlr}"
REMOTE_VERSION="${TDLR_INSTALL_VERSION:-latest}"
REMOTE_TARGET="${TDLR_INSTALL_TARGET:-auto}"
INSTALL_DIR="${TDLR_INSTALL_DIR:-}"
HAS_EXPLICIT_INSTALL_DIR=0
PROXY_PREFIX=""
SKIP_GIT_HISTORY=0
SCRIPT_PATH="${BASH_SOURCE[0]:-}"
SCRIPT_DIR=""
REPO_ROOT=""

if [[ -n "${SCRIPT_PATH}" ]]; then
  SCRIPT_DIR="$(cd -- "$(dirname -- "${SCRIPT_PATH}")" && pwd)"
  REPO_ROOT="$(cd -- "${SCRIPT_DIR}/.." 2>/dev/null && pwd || true)"
fi

if [[ -n "${INSTALL_DIR}" ]]; then
  HAS_EXPLICIT_INSTALL_DIR=1
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
      HAS_EXPLICIT_INSTALL_DIR=1
      shift 2
      ;;
    --proxy)
      PROXY_PREFIX="https://gh-proxy.com/"
      shift
      ;;
    --skip-git-history)
      SKIP_GIT_HISTORY=1
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

normalize_dir() {
  local value="$1"

  [[ -n "${value}" ]] || return 1

  value="${value%/}"
  [[ -n "${value}" ]] || value="/"

  if [[ -d "${value}" ]]; then
    (
      cd -- "${value}"
      pwd
    )
  else
    printf '%s\n' "${value}"
  fi
}

add_unique_dir() {
  local value="$1"
  local normalized
  local existing

  normalized="$(normalize_dir "${value}" 2>/dev/null || true)"
  [[ -n "${normalized}" ]] || return 0

  for existing in "${CANDIDATE_DIRS[@]}"; do
    if [[ "${existing}" == "${normalized}" ]]; then
      return 0
    fi
  done

  CANDIDATE_DIRS+=("${normalized}")
}

dirs_from_path_env() {
  local path_value="$1"
  local entry
  local entries

  [[ -n "${path_value}" ]] || return 0

  IFS=':' read -r -a entries <<< "${path_value}"
  for entry in "${entries[@]}"; do
    [[ -n "${entry}" ]] || continue
    if [[ -f "${entry}/${BIN_NAME}" ]]; then
      add_unique_dir "${entry}"
    fi
  done
}

legacy_dirs_from_git() {
  local script_paths
  local commit
  local script_path
  local content
  local line

  if [[ "${SKIP_GIT_HISTORY}" -eq 1 ]]; then
    return 0
  fi

  if [[ -z "${REPO_ROOT}" || ! -d "${REPO_ROOT}/.git" ]]; then
    return 0
  fi

  if ! command -v git >/dev/null 2>&1; then
    return 0
  fi

  script_paths=(
    "install/install.sh"
    "install/install_local.sh"
    "install/install_daily.sh"
    "scripts/install.sh"
  )

  mapfile -t commits < <(
    cd "${REPO_ROOT}" &&
      git log --format=%H -- "${script_paths[@]}" 2>/dev/null | awk '!seen[$0]++'
  )

  for commit in "${commits[@]}"; do
    for script_path in "${script_paths[@]}"; do
      if ! content="$(cd "${REPO_ROOT}" && git show "${commit}:${script_path}" 2>/dev/null)"; then
        continue
      fi

      while IFS= read -r line; do
        if [[ "${line}" =~ ^LOCATION=\"([^\"]+)\" ]]; then
          add_unique_dir "${BASH_REMATCH[1]}"
        fi
      done <<< "${content}"
    done
  done
}

resolve_upgrade_install_dir() {
  local resolved_command=""
  local candidate_dir
  local matched_dirs=()
  local extra_dir
  local fallback_dir
  local explicit_dir

  CANDIDATE_DIRS=()

  if [[ -n "${INSTALL_DIR}" ]]; then
    add_unique_dir "${INSTALL_DIR}"
  fi

  if [[ "${HAS_EXPLICIT_INSTALL_DIR}" -eq 0 ]]; then
    if command -v "${BIN_NAME}" >/dev/null 2>&1; then
      resolved_command="$(command -v "${BIN_NAME}" || true)"
      if [[ -n "${resolved_command}" && -f "${resolved_command}" ]]; then
        add_unique_dir "$(dirname -- "${resolved_command}")"
      fi
    fi

    add_unique_dir "$(default_install_dir)"
    dirs_from_path_env "${PATH}"
    legacy_dirs_from_git
    add_unique_dir "/usr/local/bin"
  fi

  for candidate_dir in "${CANDIDATE_DIRS[@]}"; do
    if [[ -f "${candidate_dir}/${BIN_NAME}" ]]; then
      matched_dirs+=("${candidate_dir}")
    fi
  done

  if [[ "${#matched_dirs[@]}" -eq 0 ]]; then
    if [[ "${HAS_EXPLICIT_INSTALL_DIR}" -eq 1 ]]; then
      fallback_dir="${INSTALL_DIR%/}"
      [[ -n "${fallback_dir}" ]] || fallback_dir="/"
      explicit_dir="$(normalize_dir "${INSTALL_DIR}" 2>/dev/null || printf '%s\n' "${fallback_dir}")"
      printf '[tdlr] no existing install found in %s; installing there\n' "${explicit_dir}" >&2
      printf '%s\n' "${explicit_dir}"
      return 0
    fi

    printf '[tdlr] no installed binary found in detected directories. Use --install-dir to target a directory or run the installer first.\n' >&2
    return 1
  fi

  if [[ "${#matched_dirs[@]}" -gt 1 ]]; then
    printf '[tdlr] detected multiple install directories; upgrading %s\n' "${matched_dirs[0]}" >&2
    for extra_dir in "${matched_dirs[@]:1}"; do
      printf '[tdlr] additional installed copy detected at %s\n' "${extra_dir}" >&2
    done
  else
    printf '[tdlr] upgrading %s\n' "${matched_dirs[0]}" >&2
  fi

  printf '%s\n' "${matched_dirs[0]}"
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
  local candidate

  if [[ -n "${SCRIPT_DIR}" ]]; then
    candidates+=("${SCRIPT_DIR}/${BIN_NAME}")
  fi

  if [[ -n "${REPO_ROOT}" ]]; then
    candidates+=("${REPO_ROOT}/target/release/${BIN_NAME}")
    candidates+=("${REPO_ROOT}/target/debug/${BIN_NAME}")
  fi

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
          printf '[tdlr] unsupported architecture for remote upgrade: %s\n' "$(uname -m)" >&2
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
          printf '[tdlr] unsupported architecture for remote upgrade: %s\n' "$(uname -m)" >&2
          exit 1
          ;;
      esac
      ;;
    *)
      printf '[tdlr] unsupported operating system for remote upgrade: %s\n' "$(uname -s)" >&2
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
  tmp_dir="$(mktemp -d "${TMPDIR:-/tmp}/tdlr-upgrade.XXXXXX")"

  printf '[tdlr] downloading %s\n' "${url}" >&2

  if command -v curl >/dev/null 2>&1; then
    curl -fsSL "${url}" -o "${tmp_dir}/package.tar.gz"
  elif command -v wget >/dev/null 2>&1; then
    wget -qO "${tmp_dir}/package.tar.gz" "${url}"
  else
    printf '[tdlr] curl or wget is required for remote upgrade.\n' >&2
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

TARGET_INSTALL_DIR="$(resolve_upgrade_install_dir)"
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
    printf '[tdlr] remote upgrade is available only when this script runs in remote mode.\n' >&2
    exit 1
  fi
else
  SOURCE_BINARY="$(download_remote_binary || true)"
  if [[ -z "${SOURCE_BINARY}" ]]; then
    exit 1
  fi
fi

mkdir -p "${TARGET_INSTALL_DIR}"
INSTALL_PATH="${TARGET_INSTALL_DIR}/${BIN_NAME}"

if [[ "$(absolute_file_path "${SOURCE_BINARY}")" != "$(absolute_file_path "${INSTALL_PATH}")" ]]; then
  cp "${SOURCE_BINARY}" "${INSTALL_PATH}"
fi

chmod 755 "${INSTALL_PATH}"

if [[ "${INSTALL_MODE}" == "remote" ]]; then
  rm -rf "$(dirname -- "$(absolute_file_path "${SOURCE_BINARY}")")"
fi

printf '[tdlr] upgraded %s\n' "${INSTALL_PATH}"
persist_path_entry "${TARGET_INSTALL_DIR}"
printf '[tdlr] run "%s --help" to get started\n' "${BIN_NAME}"

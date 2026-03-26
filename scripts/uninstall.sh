#!/usr/bin/env bash
set -euo pipefail

BIN_NAME="tdlr"
INSTALL_DIR="${TDLR_INSTALL_DIR:-}"
HAS_EXPLICIT_INSTALL_DIR=0
KEEP_PATH=0
SKIP_GIT_HISTORY=0
REMOVE_USER_DATA=0
KEEP_USER_DATA=0
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
    --install-dir)
      INSTALL_DIR="$2"
      HAS_EXPLICIT_INSTALL_DIR=1
      shift 2
      ;;
    --keep-path)
      KEEP_PATH=1
      shift
      ;;
    --skip-git-history)
      SKIP_GIT_HISTORY=1
      shift
      ;;
    --remove-user-data)
      REMOVE_USER_DATA=1
      shift
      ;;
    --keep-user-data)
      KEEP_USER_DATA=1
      shift
      ;;
    *)
      printf '[tdlr] unknown flag: %s\n' "$1" >&2
      exit 1
      ;;
  esac
done

if [[ "${REMOVE_USER_DATA}" -eq 1 && "${KEEP_USER_DATA}" -eq 1 ]]; then
  printf '[tdlr] cannot specify both --remove-user-data and --keep-user-data\n' >&2
  exit 1
fi

default_install_dir() {
  printf '%s\n' "${HOME}/.local/bin"
}

default_user_data_dir() {
  if [[ -n "${XDG_CONFIG_HOME:-}" ]]; then
    printf '%s\n' "${XDG_CONFIG_HOME}/tdlr"
  elif [[ "${OSTYPE:-}" == darwin* ]]; then
    printf '%s\n' "${HOME}/Library/Application Support/tdlr"
  else
    printf '%s\n' "${HOME}/.config/tdlr"
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

is_owned_install_dir() {
  local dir="$1"
  local parent

  parent="$(dirname -- "${dir}")"

  if [[ "$(basename -- "${dir}")" == "tdlr" ]]; then
    return 0
  fi

  if [[ "$(basename -- "${dir}")" == "bin" && "$(basename -- "${parent}")" == "tdlr" ]]; then
    return 0
  fi

  return 1
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

profile_files() {
  if [[ -n "${TDLR_PROFILE_FILE:-}" ]]; then
    printf '%s\n' "${TDLR_PROFILE_FILE}"
    return 0
  fi

  printf '%s\n' "${HOME}/.bashrc"
  printf '%s\n' "${HOME}/.zshrc"
  printf '%s\n' "${HOME}/.profile"
}

remove_path_entry_from_file() {
  local file="$1"
  local entry="$2"
  local temp_file
  local status=0

  [[ -f "${file}" ]] || return 0

  temp_file="$(mktemp "${TMPDIR:-/tmp}/tdlr-uninstall.XXXXXX")"

  awk -v target="export PATH=\"${entry}:\$PATH\"" '
    BEGIN {
      pending_comment = ""
      changed = 0
    }
    {
      if ($0 == "# tdlr installer") {
        pending_comment = $0
        next
      }

      if ($0 == target) {
        pending_comment = ""
        changed = 1
        next
      }

      if (pending_comment != "") {
        print pending_comment
        pending_comment = ""
      }

      print
    }
    END {
      if (pending_comment != "") {
        print pending_comment
      }
      exit(changed ? 10 : 0)
    }
  ' "${file}" > "${temp_file}" || status=$?

  case "${status:-0}" in
    10)
      mv "${temp_file}" "${file}"
      printf '[tdlr] removed PATH entry from %s\n' "${file}"
      ;;
    0)
      rm -f "${temp_file}"
      ;;
    *)
      rm -f "${temp_file}"
      return 1
      ;;
  esac
}

remove_binary_from_dir() {
  local dir="$1"
  local binary_path="${dir}/${BIN_NAME}"

  if [[ ! -e "${binary_path}" ]]; then
    return 1
  fi

  rm -f "${binary_path}"
  printf '[tdlr] removed %s\n' "${binary_path}"
  REMOVED_BINARIES+=("${binary_path}")
  return 0
}

remove_empty_dir_if_owned() {
  local dir="$1"
  local parent

  [[ -d "${dir}" ]] || return 0
  is_owned_install_dir "${dir}" || return 0

  if find "${dir}" -mindepth 1 -print -quit 2>/dev/null | grep -q .; then
    return 0
  fi

  rmdir "${dir}" 2>/dev/null || return 0
  printf '[tdlr] removed empty directory %s\n' "${dir}"

  parent="$(dirname -- "${dir}")"
  if [[ "$(basename -- "${dir}")" == "bin" && "$(basename -- "${parent}")" == "tdlr" ]]; then
    if [[ -d "${parent}" ]] && ! find "${parent}" -mindepth 1 -print -quit 2>/dev/null | grep -q .; then
      rmdir "${parent}" 2>/dev/null || true
      if [[ ! -d "${parent}" ]]; then
        printf '[tdlr] removed empty directory %s\n' "${parent}"
      fi
    fi
  fi
}

should_prompt_for_user_data() {
  [[ -t 0 && -t 1 ]]
}

confirm_remove_user_data() {
  local dir="$1"
  local answer

  [[ -d "${dir}" ]] || return 1

  if [[ "${REMOVE_USER_DATA}" -eq 1 ]]; then
    return 0
  fi

  if [[ "${KEEP_USER_DATA}" -eq 1 ]]; then
    printf '[tdlr] preserved user data at %s\n' "${dir}"
    return 1
  fi

  if ! should_prompt_for_user_data; then
    printf '[tdlr] preserved user data at %s (non-interactive mode). Use --remove-user-data to delete it.\n' "${dir}"
    return 1
  fi

  printf "[tdlr] remove user data at '%s'? This deletes auth sessions and account metadata [y/N] " "${dir}"
  read -r answer
  case "${answer}" in
    [yY] | [yY][eE][sS])
      return 0
      ;;
    *)
      return 1
      ;;
  esac
}

remove_user_data_dir() {
  local dir="$1"

  [[ -d "${dir}" ]] || return 0

  rm -rf -- "${dir}"
  printf '[tdlr] removed user data directory %s\n' "${dir}"
}

update_current_shell_path() {
  local filtered=()
  local entry
  local entries

  IFS=':' read -r -a entries <<< "${PATH}"
  for entry in "${entries[@]}"; do
    local keep=1
    local candidate

    for candidate in "${CANDIDATE_DIRS[@]}"; do
      if [[ "${entry%/}" == "${candidate%/}" ]]; then
        keep=0
        break
      fi
    done

    if [[ "${keep}" -eq 1 ]]; then
      filtered+=("${entry}")
    fi
  done

  PATH="$(IFS=:; printf '%s' "${filtered[*]}")"
  export PATH
}

CANDIDATE_DIRS=()
REMOVED_BINARIES=()

if [[ -n "${INSTALL_DIR}" ]]; then
  add_unique_dir "${INSTALL_DIR}"
fi

if [[ "${HAS_EXPLICIT_INSTALL_DIR}" -eq 0 ]]; then
  add_unique_dir "$(default_install_dir)"

  if command -v "${BIN_NAME}" >/dev/null 2>&1; then
    resolved_command=""
    resolved_command="$(command -v "${BIN_NAME}" || true)"
    if [[ -n "${resolved_command}" && -f "${resolved_command}" ]]; then
      add_unique_dir "$(dirname -- "${resolved_command}")"
    fi
  fi

  dirs_from_path_env "${PATH}"
  legacy_dirs_from_git
  add_unique_dir "/usr/local/bin"
fi

if [[ "${#CANDIDATE_DIRS[@]}" -eq 0 ]]; then
  printf '[tdlr] no candidate install directories found.\n'
  exit 0
fi

found_any=0
for candidate_dir in "${CANDIDATE_DIRS[@]}"; do
  if remove_binary_from_dir "${candidate_dir}"; then
    found_any=1
  fi

  if [[ "${KEEP_PATH}" -eq 0 ]]; then
    while IFS= read -r profile_file; do
      remove_path_entry_from_file "${profile_file}" "${candidate_dir}"
    done < <(profile_files)
  fi

  remove_empty_dir_if_owned "${candidate_dir}"
done

if [[ "${KEEP_PATH}" -eq 0 ]]; then
  update_current_shell_path
  printf '[tdlr] removed matching PATH entries from shell profile files when present\n'
fi

USER_DATA_DIR="$(default_user_data_dir)"
if [[ -d "${USER_DATA_DIR}" ]]; then
  if confirm_remove_user_data "${USER_DATA_DIR}"; then
    remove_user_data_dir "${USER_DATA_DIR}"
  fi
elif [[ "${REMOVE_USER_DATA}" -eq 1 ]]; then
  printf '[tdlr] no user data directory found at %s\n' "${USER_DATA_DIR}"
fi

if [[ "${found_any}" -eq 0 ]]; then
  printf '[tdlr] no installed binary found in detected directories.\n'
else
  printf '[tdlr] uninstall complete\n'
fi

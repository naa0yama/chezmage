#!/usr/bin/env bash
# Generate Jaeger UI config JS from template, replacing {{PROJECT_NAME}}
# with the package name from Cargo.toml, or the git repository name as fallback.
# Must be run from the project root (mise tasks guarantee this).
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_NAME=$(grep -A5 '\[package\]' Cargo.toml 2>/dev/null | grep '^name' | cut -d'"' -f2 || true)
if [[ -z "${PROJECT_NAME}" ]]; then
  PROJECT_NAME=$(basename -s .git "$(git remote get-url origin 2>/dev/null)" || true)
fi
if [[ -z "${PROJECT_NAME}" ]]; then
  echo "Error: could not determine project name from Cargo.toml or git remote" >&2
  exit 1
fi
# Escape sed special characters in project name (& \ |)
ESCAPED_NAME=$(printf '%s' "${PROJECT_NAME}" | sed 's/[&\|]/\\&/g')
sed "s|{{PROJECT_NAME}}|${ESCAPED_NAME}|" "${SCRIPT_DIR}/jaeger-ui.js" > "${SCRIPT_DIR}/.jaeger-ui-generated.js"

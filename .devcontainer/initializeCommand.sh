#!/usr/bin/env bash
set -euxo pipefail

# dirs from mounts
mkdir -p \
	~/.claude/ \
	~/.config/gh \
	~/.gitconfig.d

# files from mounts
touch \
	~/.claude.json \
	~/.claude/.config.json \
	~/.gitconfig

# Write MISE_GITHUB_TOKEN to file for Docker build secret (see devcontainer.json)
_token_file="/tmp/.devcontainer-github-token"
if [ -n "${MISE_GITHUB_TOKEN:-}" ]; then
	(set +x; echo "${MISE_GITHUB_TOKEN}" > "${_token_file}")
else
	: >"${_token_file}"
fi

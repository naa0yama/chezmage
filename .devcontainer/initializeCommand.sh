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

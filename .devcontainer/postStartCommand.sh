#!/usr/bin/env bash
set -euo pipefail

echo "Validating mounted files and directories..."

# List of expected mounted files and directories (optional)
EXPECTED_MOUNTS=(
	"$HOME/.claude.json"
	"$HOME/.claude/"
)

validation_failed=false

# Check each expected mount
for mount_path in "${EXPECTED_MOUNTS[@]}"; do
	if [[ ! -e "$mount_path" ]]; then
		echo -e "\e[33mWARNING: Mount target not found: $mount_path\e[0m"
		validation_failed=true
	else
		echo "✓ Mount validated: $mount_path"
	fi
done

if [ "$validation_failed" = true ]; then
	echo ""
	echo -e "\e[33m================================================================================\e[0m"
	echo -e "\e[33m>>>                                WARNING                                   <<<\e[0m"
	echo -e "\e[33m>>>\t一部のマウントが見つかりませんが、開発は続行可能です。\e[0m"
	echo -e "\e[33m>>>\t必要に応じて devcontainer.json の mounts を確認してください。\e[0m"
	echo -e "\e[33m>>>\ttarget にはマウント先の full path が含まれるためユーザー名を変更した\e[0m"
	echo -e "\e[33m>>>\t場合修正が必要です。\e[0m"
	echo -e "\e[33m================================================================================\e[0m"
	echo ""
else
	echo "All mounts validated successfully!"
fi

# mise bootstrap: install into shared volume if not already present
export PATH="$HOME/.local/bin:$PATH"
if ! command -v mise > /dev/null 2>&1; then
	echo "Installing mise..."
	curl -fsSL --retry 3 --retry-delay 2 --retry-connrefused https://mise.jdx.dev/install.sh | sh
fi
mise --version

chmod +x .githooks/*
git config --local --unset core.hookspath || true
mise trust -y /app
mise settings add trusted_config_paths /app
mise install

echo "Installing Claude Code and OpenObserve in parallel..."
mise run claudecode:install &
mise run o2:install &
wait

echo "Starting OpenObserve..."
mise run o2

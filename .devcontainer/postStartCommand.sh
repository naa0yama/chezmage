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

chmod +x .githooks/*
mise trust /app/mise.toml
mise install

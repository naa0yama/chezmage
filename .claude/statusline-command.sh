#!/bin/sh
# Claude Code status line: shows model and context usage

input=$(cat)

model=$(echo "$input" | jq -r '.model.display_name // "unknown"')

used=$(echo "$input" | jq -r '.context_window.used_percentage // empty')
remaining=$(echo "$input" | jq -r '.context_window.remaining_percentage // empty')

if [ -n "$used" ] && [ -n "$remaining" ]; then
  printf "%s | ctx: %s%% used / %s%% left" "$model" "$used" "$remaining"
else
  printf "%s | ctx: --" "$model"
fi

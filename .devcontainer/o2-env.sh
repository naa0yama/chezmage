#!/usr/bin/env bash
# OpenObserve configuration for local development (ephemeral, no persistence)
export ZO_ROOT_USER_EMAIL="dev@o2.test"
export ZO_ROOT_USER_PASSWORD="dev"
export ZO_LOCAL_MODE=true
export ZO_LOCAL_MODE_STORAGE="disk"
export ZO_DATA_DIR="/tmp/openobserve-data"
export ZO_HTTP_PORT=5080
export ZO_GRPC_PORT=5081
export ZO_TELEMETRY_ENABLED=false
export ZO_TRACING_ENABLED=false
export ZO_PROFILING_ENABLED=false
export ZO_PROMETHEUS_ENABLED=false
export ZO_COMPACT_ENABLED=false
export ZO_MEMORY_CACHE_MAX_SIZE=256
export ZO_TELEMETRY_URL=""

# AI/MCP support
export O2_AI_ENABLED=true
export O2_TOOL_API_URL="http://localhost:5080"

# OTLP auth header derived from credentials above (used by OTel SDK via mise _.source)
_o2_auth=$(echo -n "${ZO_ROOT_USER_EMAIL}:${ZO_ROOT_USER_PASSWORD}" | base64 -w0)
export OTEL_EXPORTER_OTLP_HEADERS="Authorization=Basic ${_o2_auth}"
unset _o2_auth

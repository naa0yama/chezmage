# syntax=docker/dockerfile:1@sha256:b6afd42430b15f2d2a4c5a02b919e98a525b785b1aaff16747d2f623364e39b6
#- -------------------------------------------------------------------------------------------------
#- Global
#-
ARG DEBIAN_FRONTEND=noninteractive \
	TZ=${TZ:-Asia/Tokyo} \
	USER_NAME=cuser \
	USER_UID=${USER_UID:-60001} \
	USER_GID=${USER_GID:-${USER_UID}}

## renovate: datasource=github-releases packageName=rui314/mold versioning=semver automerge=true
ARG MOLD_VERSION=v2.40.4

# Rust tools
## renovate: datasource=github-tags packageName=matthiaskrgr/cargo-cache versioning=semver automerge=true
ARG CACHE_VERSION=0.8.3
## renovate: datasource=github-tags packageName=regexident/cargo-modules versioning=semver automerge=true
ARG MODULES_VERSION=v0.25.0
## renovate: datasource=github-releases packageName=mozilla/sccache versioning=semver automerge=true
ARG SCCACHE_VERSION=v0.14.0
## renovate: datasource=github-releases packageName=ziglang/zig versioning=semver automerge=true
ARG ZIG_VERSION=0.15.1
## renovate: datasource=github-releases packageName=rust-cross/cargo-zigbuild versioning=semver automerge=true
ARG ZIGBUILD_VERSION=v0.21.8

# retry dns and some http codes that might be transient errors
ARG CURL_OPTS="-sfSL --retry 3 --retry-delay 2 --retry-connrefused"


#- -------------------------------------------------------------------------------------------------
#- Builder Base
#-
FROM rust:1.92.0-trixie@sha256:f58923369ba295ae1f60bc49d03f2c955a5c93a0b7d49acfb2b2a65bebaf350d AS builder-base
ARG CACHE_VERSION \
	CURL_OPTS \
	DEBIAN_FRONTEND \
	MODULES_VERSION \
	MOLD_VERSION \
	SCCACHE_VERSION \
	ZIG_VERSION \
	ZIGBUILD_VERSION \
	USER_NAME \
	USER_UID \
	USER_GID \
	TZ

ENV LANG=C.UTF-8 LC_ALL=C.UTF-8

SHELL [ "/bin/bash", "-c" ]

RUN echo "**** set Timezone ****" && \
	set -euxo pipefail && \
	ln -snf /usr/share/zoneinfo/${TZ} /etc/localtime && echo ${TZ} > /etc/timezone

RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
	--mount=type=cache,target=/var/lib/apt,sharing=locked \
	\
	echo "**** Dependencies ****" && \
	rm -f /etc/apt/apt.conf.d/docker-clean && \
	echo 'Binary::apt::APT::Keep-Downloaded-Packages "true";' > /etc/apt/apt.conf.d/keep-cache && \
	echo "**** Dependencies ****" && \
	set -euxo pipefail && \
	apt-get -y update && \
	apt-get -y upgrade && \
	apt-get -y install --no-install-recommends \
	bash \
	bash-completion \
	ca-certificates \
	curl \
	git \
	gnupg \
	jq \
	musl-tools \
	nano \
	sudo \
	wget

RUN echo "**** Create user ****" && \
	set -euxo pipefail && \
	groupadd --gid "${USER_GID}" "${USER_NAME}" && \
	useradd -s /bin/bash --uid "${USER_UID}" --gid "${USER_GID}" -m "${USER_NAME}" && \
	echo "${USER_NAME}:password" | chpasswd && \
	passwd -d "${USER_NAME}"

RUN echo "**** Add sudo user ****" && \
	set -euxo pipefail && \
	echo -e "${USER_NAME}\tALL=(ALL) NOPASSWD:ALL" > "/etc/sudoers.d/${USER_NAME}"

RUN echo "**** Install mold ****" && \
	set -euxo pipefail && \
	_release_data="$(curl ${CURL_OPTS} -H 'User-Agent: builder/1.0' \
	https://api.github.com/repos/rui314/mold/releases/tags/${MOLD_VERSION})" && \
	_asset="$(echo "$_release_data" | jq -r '.assets[] | select(.name | endswith("-x86_64-linux.tar.gz"))')" && \
	_download_url="$(echo "$_asset" | jq -r '.browser_download_url')" && \
	_digest="$(echo "$_asset" | jq -r '.digest')" && \
	_sha256="${_digest#sha256:}" && \
	_filename="$(basename "$_download_url")" && \
	curl ${CURL_OPTS} -H 'User-Agent: builder/1.0' -o "./${_filename}" "${_download_url}" && \
	echo "${_sha256}  ${_filename}" | sha256sum -c - && \
	tar -xvf "./${_filename}" --strip-components 1 -C /usr && \
	type -p mold && \
	rm -rf "./${_filename}"

RUN echo "**** Rust tool sccache ****" && \
	set -euxo pipefail && \
	_download_url="$(curl ${CURL_OPTS} -H 'User-Agent: builder/1.0' \
	https://api.github.com/repos/mozilla/sccache/releases/tags/${SCCACHE_VERSION} | \
	jq -r '.assets[] | select(.name | startswith("sccache-v") and endswith("-x86_64-unknown-linux-musl.tar.gz")) | .browser_download_url')" && \
	_filename="$(basename "$_download_url")" && \
	_tmpdir=$(mktemp -q -d) && \
	curl ${CURL_OPTS} -H 'User-Agent: builder/1.0' -o "./${_filename}" "${_download_url}" && \
	tar -xvf "./${_filename}" --strip-components 1 -C "${_tmpdir}" && \
	ls -lah "${_tmpdir}" && \
	cp -av "${_tmpdir}/sccache" /usr/local/bin/ && \
	type -p sccache && \
	rm -rf "./${_filename}" "${_tmpdir}"

RUN echo "**** Rust tool zig ****" && \
	set -euxo pipefail && \
	_filename="zig-x86_64-linux-${ZIG_VERSION}.tar.xz" && \
	_tmpdir=$(mktemp -q -d) && \
	mkdir -p /usr/local/zig && \
	curl ${CURL_OPTS} -H 'User-Agent: builder/1.0' -o "./${_filename}" \
	"https://ziglang.org/download/${ZIG_VERSION}/zig-x86_64-linux-${ZIG_VERSION}.tar.xz" && \
	tar -xf "./${_filename}" --strip-components 1 -C "/usr/local/zig/" && \
	ls -lah /usr/local/zig && \
	rm -rf "./${_filename}" "${_tmpdir}"

RUN echo "**** Rust tool cargo-zigbuild ****" && \
	set -euxo pipefail && \
	_release_data="$(curl ${CURL_OPTS} -H 'User-Agent: builder/1.0' \
	https://api.github.com/repos/rust-cross/cargo-zigbuild/releases/tags/${ZIGBUILD_VERSION})" && \
	_asset="$(echo "$_release_data" | jq -r '.assets[] | select(.name | endswith("x86_64-unknown-linux-musl.tar.xz"))')" && \
	_download_url="$(echo "$_asset" | jq -r '.browser_download_url')" && \
	_digest="$(echo "$_asset" | jq -r '.digest')" && \
	_sha256="${_digest#sha256:}" && \
	_filename="$(basename "$_download_url")" && \
	curl ${CURL_OPTS} -H 'User-Agent: builder/1.0' -o "./${_filename}" "${_download_url}" && \
	echo "${_sha256}  ${_filename}" | sha256sum -c - && \
	tar -xvf "./${_filename}" --strip-components 1 -C /usr/local/bin/ && \
	type -p cargo-zigbuild && \
	rm -rf "./${_filename}"

RUN --mount=type=bind,source=rust-toolchain.toml,target=/rust-toolchain.toml \
	\
	echo "**** Rust component ****" && \
	set -euxo pipefail && \
	cargo -V

# User level settings
USER ${USER_NAME}
ENV CARGO_HOME=/home/${USER_NAME}/.cargo

RUN echo "**** Directory Create ****" && \
	set -euxo pipefail && \
	mkdir -p ~/.local ~/.config

RUN echo "**** PATH add zig ****" && \
	set -euxo pipefail && \
	echo -e "# Add PATH ziglang\nexport PATH="/usr/local/zig:\$PATH"" >> ~/.bashrc && \
	exec ${SHELL} -l && \
	zig version

RUN echo "**** Create ${CARGO_HOME} ****" && \
	set -euxo pipefail && \
	mkdir -p "${CARGO_HOME}"

RUN --mount=type=cache,target=/home/cuser/.cache/sccache,sharing=locked,uid=${USER_UID},gid=${USER_GID} \
	--mount=type=cache,target=/home/cuser/.cargo/registry,sharing=locked,uid=${USER_UID},gid=${USER_GID} \
	\
	echo "**** Rust tools ****" && \
	set -euxo pipefail && \
	cargo install --locked \
	cargo-cache@${CACHE_VERSION} \
	cargo-modules@${MODULES_VERSION#v} \
	&& \
	cargo cache --version && \
	cargo modules --version

RUN echo "**** Rust bash-completion ****" && \
	set -euxo pipefail && \
	echo "export PATH="\$CARGO_HOME/bin:\$PATH"" >> ~/.bashrc && \
	\
	mkdir -p                         /home/${USER_NAME}/.local/share/bash-completion/completions && \
	rustup completions bash cargo  > /home/${USER_NAME}/.local/share/bash-completion/completions/cargo && \
	rustup completions bash rustup > /home/${USER_NAME}/.local/share/bash-completion/completions/rustup

USER root


#- -------------------------------------------------------------------------------------------------
#- Development
#-
FROM builder-base AS development
ARG CURL_OPTS \
	DEBIAN_FRONTEND \
	USER_NAME

RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
	--mount=type=cache,target=/var/lib/apt,sharing=locked \
	\
	echo "**** Dependencies ****" && \
	set -euxo pipefail && \
	apt-get -y install --no-install-recommends \
	shellcheck

# User level settings
USER ${USER_NAME}
RUN echo "**** Install mise ****" && \
	set -euxo pipefail && \
	curl https://mise.jdx.dev/install.sh | sh && \
	~/.local/bin/mise --version

COPY --chown=${USER_NAME}:${USER_NAME} mise.toml /tmp/mise.toml
RUN echo "**** Install tools via mise ****" && \
	set -euxo pipefail && \
	cd /tmp && \
	~/.local/bin/mise trust -y /tmp/mise.toml && \
	~/.local/bin/mise install -y && \
	~/.local/bin/mise trust -y --untrust /tmp/mise.toml && \
	rm /tmp/mise.toml

RUN <<EOF
echo "**** add '~/.bashrc mise and claude code ****"
set -euxo pipefail

cat <<- '_DOC_' >> ~/.bashrc
# mise
eval "$(~/.local/bin/mise activate bash)"

# This requires bash-completion to be installed
if [ ! -f "${HOME}/.local/share/bash-completion/completions/mise" ]; then
	~/.local/bin/mise use -g usage
	mkdir -p "${HOME}/.local/share/bash-completion/completions/"
	~/.local/bin/mise completion bash --include-bash-completion-lib > "${HOME}/.local/share/bash-completion/completions/mise"
fi

# Claude Code
export PATH="$HOME/.local/bin:$PATH"
alias cc="claude --dangerously-skip-permissions"

_DOC_
EOF

# Ref: https://docs.anthropic.com/en/docs/claude-code/setup#native-binary-installation-beta
RUN echo "**** Install Claude Code ****" && \
	set -euxo pipefail && \
	curl -fsSL https://claude.ai/install.sh | bash && \
	exec ${SHELL} -l && \
	claude --version && \
	type cc


#- -------------------------------------------------------------------------------------------------
#- Production
#-
#FROM debian:bullseye-slim
#ARG DEBIAN_FRONTEND \
#	TZ
#
#SHELL [ "/bin/bash", "-c" ]
#
#RUN echo "**** set Timezone ****" && \
#	set -euxo pipefail && \
#	ln -snf /usr/share/zoneinfo/${TZ} /etc/localtime && echo ${TZ} > /etc/timezone
#
#RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
#	--mount=type=cache,target=/var/lib/apt,sharing=locked \
#	\
#	echo "**** Dependencies ****" && \
#	set -euxo pipefail && \
#	apt-get -y install --no-install-recommends \
#	bash \
#	ca-certificates
#
##COPY --from=development /usr/local/cargo/bin/myapp /usr/local/bin/myapp
#
#SHELL [ "/bin/sh", "-c" ]
##CMD ["myapp"]

# vim: set filetype=dockerfile:

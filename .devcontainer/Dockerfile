ARG VARIANT=bullseye
FROM mcr.microsoft.com/vscode/devcontainers/rust:1-${VARIANT}

RUN apt-get update && export DEBIAN_FRONTEND=noninteractive \
    && apt-get -y install --no-install-recommends build-essential libssl-dev pkg-config cmake wabt

RUN rustup default nightly

RUN chown -R vscode /usr/local/rustup
RUN chown -R vscode /usr/local/cargo
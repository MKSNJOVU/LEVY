# Pin the base image strictly to Rust 1.94.0 on Debian Bullseye
FROM mcr.microsoft.com/devcontainers/rust:1.94.0-bullseye

# Update the apt package index and install required system dependencies
# - protobuf-compiler: Required by the 'prost' and 'tonic' crates to compile gRPC files
# - libsqlite3-dev: Required by 'sqlx' for SQLite database operations
RUN apt-get update && export DEBIAN_FRONTEND=noninteractive \
    && apt-get -y install --no-install-recommends \
    protobuf-compiler \
    libsqlite3-dev \
    && apt-get clean -y && rm -rf /var/lib/apt/lists/*
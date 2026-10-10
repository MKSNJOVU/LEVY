# Use the modern Debian 12 (Bookworm) base image
FROM mcr.microsoft.com/devcontainers/base:bookworm

# Update the apt package index and install required system dependencies
# - protobuf-compiler: Required to compile gRPC files
# - libsqlite3-dev: Required for SQLite database operations
# - pkg-config & libssl-dev: Fixes the sqlx-cli OpenSSL compilation error
# - libwebkit2gtk-4.1-dev & others: Required by Tauri 2.0 for Linux desktop GUIs
RUN apt-get update && export DEBIAN_FRONTEND=noninteractive \
    && apt-get -y install --no-install-recommends \
    protobuf-compiler \
    libsqlite3-dev \
    pkg-config \
    libssl-dev \
    libwebkit2gtk-4.1-dev \
    build-essential \
    curl \
    wget \
    file \
    libxdo-dev \
    libayatana-appindicator3-dev \
    librsvg2-dev \
    && apt-get clean -y && rm -rf /var/lib/apt/lists/*
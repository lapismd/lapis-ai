FROM rust:1.99.0-slim-bookworm@sha256:2c3a22f0a5533ea2dd5a16627bc841228151faa2d4de2644ac9987e4a2f1f2fa AS rust
FROM ghcr.io/lapismd/lapis-ci@sha256:91341a81cc9306eed818c4c433cd29ed11d2ea64c703f32685391befe0eca503
ENV PNPM_HOME=/opt/pnpm RUSTUP_HOME=/usr/local/rustup CARGO_HOME=/cache/cargo CARGO_TARGET_DIR=/cache/target
ENV CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0
ENV PATH=/opt/tools:/usr/local/cargo/bin:/opt/pnpm:$PATH
COPY --from=rust /usr/local/cargo /usr/local/cargo
COPY --from=rust /usr/local/rustup /usr/local/rustup
RUN apt-get update && apt-get install -y --no-install-recommends git curl ca-certificates lsof build-essential python3 pkg-config libssl-dev libgtk-3-dev libwebkit2gtk-4.1-dev librsvg2-dev && rm -rf /var/lib/apt/lists/*
RUN rustup component add rustfmt clippy --toolchain 1.99.0
RUN corepack prepare pnpm@10.34.5 --activate
COPY install-tools.mjs /opt/install-tools.mjs
RUN node /opt/install-tools.mjs
RUN git config --system user.name 'Lapis AI validation' && git config --system user.email 'validation@localhost' && git config --system --add safe.directory '*'
RUN mkdir -p /input /results /work/lapis-ai
WORKDIR /work/lapis-ai

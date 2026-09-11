FROM node:22-bookworm-slim AS styles
WORKDIR /app
COPY package.json package-lock.json ./
RUN npm ci
COPY tailwind.config.cjs ./
COPY style ./style
COPY crates/web/src ./crates/web/src
RUN mkdir -p crates/web/public && npm run css

FROM rust:1.94.0-bookworm AS build
RUN rustup target add wasm32-unknown-unknown \
    && cargo install cargo-leptos --version 0.3.7 --locked
WORKDIR /app
COPY . .
COPY --from=styles /app/crates/web/public/app.css crates/web/public/app.css
RUN cargo fetch --locked \
    && cargo leptos build --release \
    && cargo build --release --locked -p etyloom-cli

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --gid 10001 etyloom \
    && useradd --uid 10001 --gid etyloom --no-create-home etyloom \
    && mkdir -p /data /app/site \
    && chown -R etyloom:etyloom /data /app
COPY --from=build /app/target/release/etyloom-web /usr/local/bin/etyloom-web
COPY --from=build /app/target/release/etyloom /usr/local/bin/etyloom
COPY --from=build /app/target/site /app/site
USER 10001:10001
WORKDIR /app
ENV LEPTOS_SITE_ADDR=0.0.0.0:3000 \
    LEPTOS_SITE_ROOT=/app/site \
    LEPTOS_OUTPUT_NAME=etyloom \
    LEPTOS_SITE_PKG_DIR=pkg \
    DATABASE_URL=sqlite:///data/etyloom.db \
    ETYLOOM_WORKERS=1 \
    RUST_LOG=info,tower_http=warn
EXPOSE 3000
VOLUME ["/data"]
HEALTHCHECK --interval=30s --timeout=5s --start-period=15s --retries=3 \
    CMD ["etyloom-web", "healthcheck"]
ENTRYPOINT ["etyloom-web"]

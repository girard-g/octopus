# ---- frontend build ----
FROM node:22-bookworm-slim AS frontend
WORKDIR /app/frontend
COPY frontend/package.json frontend/package-lock.json* ./
RUN npm install --no-fund --no-audit
COPY frontend/ ./
# vite.config.js emits to ../static (i.e. /app/static)
RUN npm run build

# ---- rust build ----
FROM rust:1-bookworm AS build
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && echo "" > src/lib.rs \
    && cargo build --release ; rm -rf src
COPY . .
RUN cargo build --release

# ---- runtime ----
FROM debian:bookworm-slim AS runtime
WORKDIR /app
# curl is required by the container healthcheck (Coolify/Docker run it inside the
# image); debian-slim ships neither curl nor wget, so install it explicitly.
RUN apt-get update && apt-get install -y ca-certificates curl && rm -rf /var/lib/apt/lists/*
COPY --from=build /app/target/release/octopus /usr/local/bin/octopus
COPY --from=build /app/migrations ./migrations
COPY --from=frontend /app/static ./static
ENV PORT=8080
EXPOSE 8080
HEALTHCHECK --interval=30s --timeout=3s --start-period=10s --retries=3 \
    CMD curl -fsS http://localhost:8080/api/health || exit 1
CMD ["octopus"]

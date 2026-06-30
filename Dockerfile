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
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=build /app/target/release/octopus /usr/local/bin/octopus
COPY --from=build /app/migrations ./migrations
COPY --from=frontend /app/static ./static
ENV PORT=8080
EXPOSE 8080
CMD ["octopus"]

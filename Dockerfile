# ---- build ----
FROM rust:1-bookworm AS build
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
# cache deps
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
COPY --from=build /app/static ./static
ENV PORT=8080
EXPOSE 8080
CMD ["octopus"]

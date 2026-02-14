FROM rustlang/rust:nightly-alpine AS chef
WORKDIR /app

RUN apk add --no-cache musl-dev
RUN cargo install cargo-chef

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --recipe-path recipe.json
# Build application
COPY . .
RUN cargo build --release --bin role-manager

FROM scratch AS runtime
WORKDIR /app
COPY --from=builder /app/target/release/role-manager /usr/local/bin/role-manager
ENTRYPOINT ["/usr/local/bin/role-manager"]

FROM rust:1.69.0-slim-buster AS chef
RUN cargo install cargo-chef --version 0.1.59 --locked
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
# Build application
COPY . .
RUN cargo build --release

FROM gcr.io/distroless/cc-debian11 AS runtime
WORKDIR /app
COPY --from=builder /app/target/release/server .
COPY app.env .env
COPY migrations .
EXPOSE 8080
CMD [ "/app/server" ]
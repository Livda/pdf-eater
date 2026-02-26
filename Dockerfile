FROM rust:1-alpine AS builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
RUN cargo build --release || true
COPY static ./static
COPY src ./src
RUN cargo build --release

FROM alpine:3.23 AS prod

WORKDIR /app
COPY static ./static
COPY --from=builder /app/target/release/pdf-eater .
EXPOSE 8080
CMD ["./pdf-eater"]

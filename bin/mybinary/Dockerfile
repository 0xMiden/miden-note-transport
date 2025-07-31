FROM rust:1.87-slim-bullseye AS builder

# TODO(template) update for the binary
RUN apt-get update && \
    apt-get -y upgrade && \
    apt-get install -y llvm clang bindgen pkg-config libssl-dev libsqlite3-dev ca-certificates && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY ./Cargo.toml .
COPY ./Cargo.lock .
COPY ./bin ./bin
COPY ./crates ./crates

# TODO(template) update for the binary
RUN cargo install --path bin/mybinary --locked

FROM debian:bullseye-slim

# Update machine & install required packages
# The installation of sqlite3 is needed for correct function of the SQLite database
# TODO(template) update for the binary
RUN apt-get update && \
    apt-get -y upgrade && \
    apt-get install -y --no-install-recommends \
    sqlite3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/local/cargo/bin/miden-mybinary /usr/local/bin/miden-mybinary

# TODO(template) update for the binary
LABEL org.opencontainers.image.authors=devops@miden.team \
    org.opencontainers.image.url=https://0xMiden.github.io/ \
    org.opencontainers.image.documentation=https://github.com/0xMiden/miden-mybinary \
    org.opencontainers.image.source=https://github.com/0xMiden/miden-mybinary \
    org.opencontainers.image.vendor=Miden \
    org.opencontainers.image.licenses=MIT

ARG CREATED
ARG VERSION
ARG COMMIT
LABEL org.opencontainers.image.created=$CREATED \
    org.opencontainers.image.version=$VERSION \
    org.opencontainers.image.revision=$COMMIT

# TODO(template) update for the binary
# Expose port
EXPOSE 8080

# Start
# TODO(template) update for the binary
CMD miden-mybinary

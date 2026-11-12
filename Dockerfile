FROM rust:1.100-bookworm AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
RUN cargo build --release -p dshr-cli

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=build /src/target/release/dshr /usr/local/bin/dshr
EXPOSE 55678
ENTRYPOINT ["dshr"]
CMD ["serve", "--bind", "0.0.0.0", "--port", "55678"]

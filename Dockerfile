FROM rust:1.72 as builder
WORKDIR /usr/src/cse138
COPY . . 
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /usr/src/cse138/target/release/pa2 /usr/local/bin/pa2

# Bypassing signature verification due to known issues with Debian repositories. Very unsafe, but I'm gay.
RUN apt-get -o Acquire::Check-Valid-Until=false -o Acquire::Check-Date=false update && \
    apt-get install -y libssl-dev && \
    rm -rf /var/lib/apt/lists/*

EXPOSE 8090
CMD ["pa2"]

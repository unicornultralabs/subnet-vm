FROM rustlang/rust:nightly-bookworm as build

WORKDIR /app

# Runtime image
RUN     apt-get update
ARG     DEBIAN_FRONTEND=noninteractive
RUN     apt-get update && \
	apt-get install -y gcc llvm clang libtool && \
	rm -rf /var/lib/apt/lists/* \
    cargo install hvm@2.0.17 

# Install nightly
RUN		rustup toolchain install nightly-2023-12-22
RUN 	rustup default nightly-2023-12-22

# Copy source code & install
COPY    . .
RUN     RUSTFLAGS="-C target-cpu=native" cargo build --release
RUN		cp /app/target/release/subnet_vm /usr/local/bin/subnet_vm

FROM    ubuntu:22.04
RUN     apt-get update && apt-get -y install libssl3
COPY    --from=build /app/target/release/subnet_vm /usr/local/bin/subnet_vm
WORKDIR /

ENTRYPOINT     ["subnet_vm"]
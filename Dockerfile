FROM rust:1.65

WORKDIR /privadex
COPY Cargo.lock Cargo.toml rust-toolchain.toml .
COPY dex_aggregator dex_aggregator

ENV PATH=/home/user/.cargo/bin:$PATH
CMD /bin/bash


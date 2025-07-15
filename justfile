alias b := build
alias br := build-release
alias t := test

build:
    cargo build

build-release:
    cargo build --release

test:
    cargo test --workspace

bins:
    cargo build --bins

bins-release:
    cargo build --bins --release

watch:
    cargo watch -x "build --bins"

install:
    cargo install --path .

+++
title = "Installation"
weight = 10
+++

Ansorum installation currently has two practical paths:

- install a prebuilt binary from the
  [GitHub releases page](https://github.com/nibzard/ansorum/releases) when an
  asset exists for your platform
- install from source with Cargo

The release workflow is configured to publish these binary targets:

- `x86_64-unknown-linux-gnu`
- `x86_64-unknown-linux-musl`
- `aarch64-unknown-linux-gnu`
- `x86_64-pc-windows-msvc`
- `x86_64-apple-darwin`
- `aarch64-apple-darwin`

## Prebuilt Binary From GitHub Releases

Use the release page when there is an uploaded asset matching your platform.
The artifact naming convention is:

```text
ansorum-<tag>-<target>.tar.gz
```

A versioned Linux example using the static `musl` binary:

```sh
VERSION=vX.Y.Z
TARGET=x86_64-unknown-linux-musl
curl -fsSLo ansorum.tar.gz \
  "https://github.com/nibzard/ansorum/releases/download/${VERSION}/ansorum-${VERSION}-${TARGET}.tar.gz"
tar -xzf ansorum.tar.gz
install -m 0755 ansorum ~/.local/bin/ansorum
ansorum --version
```

Swap `TARGET` for the asset you need on your platform.
Replace `vX.Y.Z` with a real release tag that has uploaded assets.

## From Source With Cargo

To build and install Ansorum from source, you need
[Rust and Cargo](https://www.rust-lang.org/).

Install the `ansorum` binary directly from this repository:

```sh
cargo install --locked --git https://github.com/nibzard/ansorum ansorum --bin ansorum
ansorum --version
```

Cargo installs binaries into `~/.cargo/bin/` by default.

If you need the native TLS feature set instead of the default Rust TLS setup:

```sh
cargo install --locked --no-default-features --features=native-tls \
  --git https://github.com/nibzard/ansorum ansorum --bin ansorum
ansorum --version
```

## GitHub Actions

Ansorum can be installed in CI directly from the repository with Cargo:

```yaml
jobs:
  foo:
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo install --locked --path .
      - run: ansorum --version
```

## Docker

The release workflow is intended to publish versioned container images to
GitHub Container Registry:

```sh
docker pull ghcr.io/nibzard/ansorum:vX.Y.Z
```

#### Build

```sh
docker run -u "$(id -u):$(id -g)" -v $PWD:/app --workdir /app ghcr.io/nibzard/ansorum:vX.Y.Z build
```

#### Serve

```sh
docker run -u "$(id -u):$(id -g)" -v $PWD:/app --workdir /app -p 8080:8080 ghcr.io/nibzard/ansorum:vX.Y.Z serve --interface 0.0.0.0 --port 8080 --base-url localhost
```

You can now browse http://localhost:8080.

#### Multi-stage build

Since there is no shell in the Ansorum docker image, if you want to use it
from inside a Dockerfile, use the exec form of `RUN`:

```Dockerfile
FROM ghcr.io/nibzard/ansorum:vX.Y.Z as ansorum

COPY . /project
WORKDIR /project
RUN ["ansorum", "build"]
```

FROM rust:slim-bookworm AS builder

ARG USE_GH_RELEASE=false
ARG ANSORUM_RELEASE_VERSION=latest
RUN apt-get update -y && \
  apt-get install -y pkg-config make g++ libssl-dev curl jq tar gzip

WORKDIR /app
COPY . .

RUN if [ "${USE_GH_RELEASE}" = "true" ]; then \
    if [ "${ANSORUM_RELEASE_VERSION}" = "latest" ]; then \
      export ANSORUM_VERSION=$(curl -sL https://api.github.com/repos/nibzard/ansorum/releases/latest | jq -r .name); \
    else \
      export ANSORUM_VERSION="${ANSORUM_RELEASE_VERSION}"; \
    fi && \
    curl -sL --fail --output ansorum.tar.gz https://github.com/nibzard/ansorum/releases/download/${ANSORUM_VERSION}/ansorum-${ANSORUM_VERSION}-$(uname -m)-unknown-linux-gnu.tar.gz && \
    tar -xzvf ansorum.tar.gz ansorum; \
  else \
    cargo build --release && \
    cp target/release/ansorum ansorum; \
  fi && ./ansorum --version

FROM gcr.io/distroless/cc-debian12
COPY --from=builder /app/ansorum /bin/ansorum
ENTRYPOINT [ "/bin/ansorum" ]

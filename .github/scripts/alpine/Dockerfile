FROM docker.io/library/python:3.14-alpine

RUN apk add --no-cache build-base

COPY --from=rustlang/rust:nightly-alpine /usr/local/cargo /root/.cargo
COPY --from=rustlang/rust:nightly-alpine /usr/local/rustup /root/.rustup

ENV PATH="/root/.cargo/bin:${PATH}"

COPY --from=ghcr.io/astral-sh/uv:latest /uv /bin/
COPY entrypoint.sh /entrypoint.sh

RUN chmod +x /entrypoint.sh

ENTRYPOINT ["/entrypoint.sh"]
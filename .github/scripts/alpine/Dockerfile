FROM docker.io/library/python:3.14-alpine

RUN apk add --no-cache rust cargo

COPY --from=ghcr.io/astral-sh/uv:alpine /uv /usr/local/bin/

ENV PATH="/root/.local/bin:$PATH"

COPY entrypoint.sh /entrypoint.sh

ENTRYPOINT ["/entrypoint.sh"]
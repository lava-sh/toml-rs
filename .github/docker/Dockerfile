FROM docker.io/library/python:3.13-alpine

WORKDIR /app

ENV UV_LINK_MODE=copy \
    UV_COMPILE_BYTECODE=1 \
    UV_PYTHON_DOWNLOADS=0

COPY --from=ghcr.io/astral-sh/uv:latest /uv /uvx /bin/

RUN /bin/sh -c "${CMD_}"

RUN cat <<'EOF' > /app/example.py
import toml_rs

t = """\
mimalloc = {
    version = "0.1.48",
    features = [
        "v3",
        "override",
        "local_dynamic_tls"
    ],
    optional = true,
}
"""

loads = toml_rs.loads(t, toml_version="1.1.0")

expected = {
    "mimalloc": {
        "version": "0.1.48",
        "features": ["v3", "override", "local_dynamic_tls"],
        "optional": True,
    },
}

assert loads == expected
print(toml_rs.__version__)
EOF

RUN addgroup -S user && adduser -S -G user user
RUN chown -R user:user /app
USER user

CMD ["python", "example.py"]
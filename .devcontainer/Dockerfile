FROM ghcr.io/musicalninjas/pyo3-devcontainer:latest@sha256:77d7def358fe9f828476d056a0af166b2f8bc9bf6e7a56237472bc6a5f16322d

# Zigbuild to support building for specific glibc versions
# cargo-msrv to validate MSRV
# Set default linker to mold (https://github.com/rui314/setup-mold/blob/main/action.yml#L18C109-L18C161)
USER root

RUN uv pip install --system ziglang \
    && cargo binstall -y cargo-zigbuild cargo-msrv \
    && ln -sf /usr/local/bin/mold "$(realpath /usr/bin/ld)"

USER 1000

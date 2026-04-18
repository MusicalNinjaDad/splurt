FROM ghcr.io/musicalninjas/pyo3-devcontainer:latest@sha256:77d7def358fe9f828476d056a0af166b2f8bc9bf6e7a56237472bc6a5f16322d

# cargo-zigbuild to support building for specific glibc versions
# cargo-msrv to validate MSRV
USER root

RUN uv pip install --system ziglang \
    && cargo binstall -y cargo-zigbuild cargo-msrv

USER 1000

# Notes on mold:
# ==============
# - Setting default linker to mold via symlink (https://github.com/rui314/setup-mold/blob/main/action.yml#L18C109-L18C161)
#   breaks cargo msrv checks for <1.90.0
# - Cargo zigbuild uses internal linker anyway
# - Setting mold in cargo.toml breaks CI unless we also install mold in all jobs
# - Use RUSTFLAGS="-C link-arg=-fuse-ld=/usr/sbin/mold -C linker=/usr/sbin/clang" directly for cargo build to use mold

# Base image source: https://raw.githubusercontent.com/MusicalNinjaDad/rust/refs/heads/main/Dockerfile
FROM ghcr.io/musicalninjadad/rust-devcontainer:latest@sha256:72b7aec3f0dc037f4590286207652791961e5424bdda06866f3f20de03362403

RUN sudo dnf install openssl-devel pkgconf-pkg-config
RUN cargo binstall -y cargo-xwin jaq uv
RUN cargo-xwin xwin cache windows-msvc-sysroot \
    && cargo-xwin xwin cache xwin

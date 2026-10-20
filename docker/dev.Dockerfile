# Dev container: pinned Rust toolchain + CPython 3.12 for embedded scripting.
# Built from the repo root: docker compose build
FROM python:3.12.3-slim-bookworm

ENV DEBIAN_FRONTEND=noninteractive

# System libs required by eframe/wgpu, serialport, and headless GPU tests.
# Build-time X11 libs, plus the runtime libs winit loads dynamically
# (libXcursor, libXrandr, libXi, libXinerama, libxkbcommon-x11).
# mesa-vulkan-drivers provides the lavapipe software Vulkan fallback when no
# GPU is passed to the container.
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential clang curl git pkg-config ca-certificates \
    libxkbcommon-dev libwayland-dev \
    libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev \
    libgl1-mesa-dev libudev-dev \
    libxcursor1 libxrandr2 libxi6 libxinerama1 libxkbcommon-x11-0 \
    libvulkan1 vulkan-tools mesa-vulkan-drivers \
    zenity dbus-x11 file \
  && rm -rf /var/lib/apt/lists/*

# Rust via rustup; install exactly the toolchain pinned in
# rust-toolchain.toml (parsed, not duplicated) so containers start warm.
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
    | sh -s -- -y --profile minimal --default-toolchain none
ENV PATH=/root/.cargo/bin:$PATH

COPY rust-toolchain.toml /tmp/rust-toolchain.toml
RUN rustup toolchain install \
    "$(awk -F'"' '/^channel/ {print $2}' /tmp/rust-toolchain.toml)" \
    --profile minimal --component rustfmt --component clippy \
  && rm /tmp/rust-toolchain.toml

# Python packages pinned by CI (.github/workflows/ci.yml) for the
# embedded-scripting build.
RUN pip install --no-cache-dir \
    numpy==2.1.1 scipy==1.14.1 bottleneck==1.4.0 cffi==1.17.1

ENV CARGO_TERM_COLOR=always \
    CARGO_NET_RETRY=3

WORKDIR /workspace
CMD ["bash"]

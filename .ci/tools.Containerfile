FROM centos:8
RUN dnf install -y \
      # baseline tools
      curl \
      gcc \
      glibc-devel \
      gzip \
      tar \
      unzip \

      # container tools
      buildah \
      skopeo \

      # tool dependencies
      openssl-devel \

      # cirrus build dependencies
      dbus-devel \
      && \
    dnf clean all

# qemu-user-static
COPY install-qemu.sh /install-qemu.sh

ENV QEMU_ARM_URL=https://github.com/balena-io/qemu/releases/download/v4.0.0%2Bbalena2/qemu-4.0.0.balena2-arm.tar.gz
ENV QEMU_ARM_SHA256=ae0144b8b803ddb8620b7e6d5fd68e699a97e0e9c523d283ad54fcabc0e615f8
RUN bash /install-qemu.sh $QEMU_ARM_URL $QEMU_ARM_SHA256 /usr/bin/qemu-arm-static

ENV QEMU_AARCH64_URL=https://github.com/balena-io/qemu/releases/download/v4.0.0%2Bbalena2/qemu-4.0.0.balena2-aarch64.tar.gz
ENV QEMU_AARCH64_SHA256=e98eed19f680ae0b7e5937204040653c3312ae414f89eaccddeeb706934a63e4
RUN bash /install-qemu.sh $QEMU_AARCH64_URL $QEMU_AARCH64_SHA256 /usr/bin/qemu-aarch64-static

RUN rm /install-qemu.sh

# sccache
ENV SCCACHE_URL=https://github.com/mozilla/sccache/releases/download/0.2.13/sccache-0.2.13-x86_64-unknown-linux-musl.tar.gz
ENV SCCACHE_SHA256=28a5499e340865b08b632306b435913beb590fbd7b49a3f887a623b459fabdeb

RUN mkdir sccache && \
    cd sccache && \
    curl -L -o sccache.tgz $SCCACHE_URL && \
    echo "$SCCACHE_SHA256 *sccache.tgz" | sha256sum -c && \
    tar -C /usr/bin/ -xz -f sccache.tgz --wildcards --strip-components=1 '*/sccache' && \
    cd .. && \
    rm -rf sccache

# Rust
ENV RUST_VERSION=1.47.0
RUN curl https://sh.rustup.rs -sSf | \
    CARGO_HOME=/usr/local/cargo sh -s -- -y --profile default --no-modify-path --default-toolchain $RUST_VERSION
ENV PATH=/usr/local/cargo/bin:$PATH

FROM centos:8
RUN dnf install -y \
      buildah \
      bzip2 \
      clang \
      cmake \
      curl \
      dbus-devel \
      gettext \
      git \
      gzip \
      make \
      openssl-devel \
      skopeo \
      tar \
      unzip \
      zlib-devel && \
    dnf clean all

# MSIX SDK
ENV MSIX_URL=https://github.com/microsoft/msix-packaging.git
ENV MSIX_REV=5b6d766f4dc5045dc689786f22ffe51f0f0c5801

RUN git clone $MSIX_URL msix-sdk && \
    cd msix-sdk && \
    git checkout $MSIX_REV && \
    cmake \
      -DSKIP_BUNDLES=on \
      -DUSE_VALIDATION_PARSER=on \
      -DCMAKE_TOOLCHAIN_FILE=cmake/linux.cmake \
      -DMSIX_PACK=on \
      -DMSIX_SAMPLES=off \
      -DMSIX_TESTS=off \
      -DLINUX=on \
      . && \
    make && \
    install -D -t /usr/lib64 lib/libmsix.so && \
    install -D -t /usr/bin bin/makemsix && \
    cd .. && \
    rm -rf msix-sdk

# appx
ENV APPX_URL=https://github.com/facebookarchive/fb-util-for-appx.git
ENV APPX_REV=9925835f5739f81c486d5f9f911f793d0f567a0f

RUN git clone $APPX_URL appx && \
    cd appx && \
    git checkout $APPX_REV && \
    cmake \
      -DCMAKE_INSTALL_PREFIX=/usr \
      . && \
    make install && \
    cd .. && \
    rm -rf appx

# qemu-user-static
COPY install-qemu.sh /install-qemu.sh

ENV QEMU_ARM_URL=https://github.com/balena-io/qemu/releases/download/v4.0.0%2Bbalena2/qemu-4.0.0.balena2-arm.tar.gz
ENV QEMU_ARM_SHA256=ae0144b8b803ddb8620b7e6d5fd68e699a97e0e9c523d283ad54fcabc0e615f8
RUN bash /install-qemu.sh $QEMU_ARM_URL $QEMU_ARM_SHA256 /usr/bin/qemu-arm-static

ENV QEMU_AARCH64_URL=https://github.com/balena-io/qemu/releases/download/v4.0.0%2Bbalena2/qemu-4.0.0.balena2-aarch64.tar.gz
ENV QEMU_AARCH64_SHA256=e98eed19f680ae0b7e5937204040653c3312ae414f89eaccddeeb706934a63e4
RUN bash /install-qemu.sh $QEMU_AARCH64_URL $QEMU_AARCH64_SHA256 /usr/bin/qemu-aarch64-static

RUN rm /install-qemu.sh

# Rust
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y --profile standard --default-toolchain stable
RUN rustup component add clippy rustfmt

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

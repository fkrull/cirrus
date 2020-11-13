FROM centos:8
RUN dnf install -y \
      buildah \
      bzip2 \
      clang \
      cmake \
      curl \
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

ARG IMAGE_ARCH
FROM docker.io/${IMAGE_ARCH}/debian:11-slim
# qemu binary must be mounted into build container
ARG QEMU
RUN $QEMU apt-get update && \
    $QEMU apt-get install --no-install-recommends -y \
        ca-certificates \
        libdbus-1-3 \
        openssh-client \
    && \
    $QEMU apt-get clean && \
    $QEMU rm -rf /var/lib/apt

ARG TARBALL
ADD --chown=root:root --chmod=0755 $TARBALL /usr/local/bin

ENV XDG_CONFIG_HOME=/config
ENV XDG_DATA_HOME=/data/data
ENV XDG_CACHE_HOME=/data/cache
VOLUME /data
ENTRYPOINT ["/usr/local/bin/cirrus"]
CMD ["daemon"]

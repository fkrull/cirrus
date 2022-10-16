ARG IMAGE_ARCH
FROM docker.io/${IMAGE_ARCH}/debian:11-slim
RUN apt-get update && \
    apt-get install --no-install-recommends -y ca-certificates openssh-client && \
    apt-get clean && \
    rm -rf /var/lib/apt

ARG TARBALL
ADD --chown=root:root $TARBALL /usr/local/bin

ENV XDG_CONFIG_HOME=/config
ENV XDG_DATA_HOME=/data/data
ENV XDG_CACHE_HOME=/data/cache
VOLUME /data
ENTRYPOINT ["/usr/local/bin/cirrus"]
CMD ["daemon"]

ARG IMAGE_ARCH
FROM docker.io/${IMAGE_ARCH}/alpine:3
RUN apk add --no-cache \
    ca-certificates \
    openssh-client

ARG TARBALL
ADD --chown=root:root $TARBALL /usr/local/bin

ENV XDG_CONFIG_HOME=/config
ENV XDG_DATA_HOME=/data/data
ENV XDG_CACHE_HOME=/data/cache
VOLUME /data
ENTRYPOINT ["/usr/local/bin/cirrus"]
CMD ["daemon"]

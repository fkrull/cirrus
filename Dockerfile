FROM docker.io/library/alpine:3.16.3
RUN apk add --no-cache \
    ca-certificates \
    openssh-client

ARG TARBALL
ADD --chown=root:root $TARBALL /usr/local/bin

ENV XDG_CONFIG_HOME=/config
ENV XDG_DATA_HOME=/data/data
ENV XDG_CACHE_HOME=/data/cache
ENV RUST_BACKTRACE=1
VOLUME /data
ENTRYPOINT ["/usr/local/bin/cirrus"]
CMD ["daemon"]

FROM docker.io/library/alpine:3.16@sha256:3d426b0bfc361d6e8303f51459f17782b219dece42a1c7fe463b6014b189c86d
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

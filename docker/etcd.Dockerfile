#!/usr/bin/env -S docker build ../ -t ghcr.io/satoqz/etcd:latest -f 

FROM alpine:3.19 

RUN apk add etcd \
    --no-cache \
    --repository=https://dl-cdn.alpinelinux.org/alpine/edge/testing

CMD ["etcd"]
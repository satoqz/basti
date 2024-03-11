#!/usr/bin/env -S docker build ../ -t satoqz.net/etcd:latest -f 
FROM alpine:3.19 

RUN apk add etcd etcd-ctl \
    --no-cache \
    --repository=https://dl-cdn.alpinelinux.org/alpine/edge/testing

CMD ["etcd"]

VERSION 0.8

build-bastid:
    FROM rust:alpine

    RUN apk add --no-cache musl-dev protoc

    WORKDIR /build
    COPY . .

    RUN --mount=type=cache,target=/usr/local/cargo/registry \
        cargo build --release --bin bastid

    SAVE ARTIFACT target/release/bastid

bastid:
    FROM scratch

    ENV PATH /
    ENV BASTID_LISTEN 0.0.0.0:1337

    COPY +build-bastid/bastid /
    ENTRYPOINT [ "bastid" ]

    SAVE IMAGE --push ghcr.io/satoqz/bastid:latest

etcd:
    FROM alpine:3.19

    RUN apk add etcd \
        --no-cache \
        --repository=https://dl-cdn.alpinelinux.org/alpine/edge/testing

    ENTRYPOINT [ "etcd" ]

    SAVE IMAGE --push ghcr.io/satoqz/etcd:latest

all-arm:
    BUILD --platform=linux/arm64 +bastid
    BUILD --platform=linux/arm64 +etcd

all-amd:
    BUILD --platform=linux/amd64 +bastid
    BUILD --platform=linux/amd64 +etcd

all:
    BUILD +all-arm
    BUILD +all-amd

#!/usr/bin/env -S docker build ../ -t ghcr.io/satoqz/bastid:latest -f 

FROM rust:alpine AS builder

RUN apk add --no-cache musl-dev protoc

WORKDIR /build
COPY . .

RUN cargo fetch --locked
RUN cargo install --locked --path basti-daemon

FROM scratch 

COPY --from=builder /usr/local/cargo/bin/bastid /
ENV PATH /

ENV BASTID_NAME bastid
ENV BASTID_LISTEN 0.0.0.0:1337

ENTRYPOINT [ "bastid" ]
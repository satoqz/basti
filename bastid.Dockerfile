FROM rust:alpine AS builder

RUN apk add --no-cache musl-dev

WORKDIR /build
COPY . .

RUN cargo fetch --locked
RUN cargo install --locked --path basti-daemon

FROM scratch 

COPY --from=builder /usr/local/cargo/bin/bastid /
CMD ["/bastid"]
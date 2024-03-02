FROM rust:alpine AS builder

RUN apk add --no-cache musl-dev protoc

WORKDIR /build
COPY . .

RUN cargo fetch --locked
RUN cargo install --locked --path basti-daemon

FROM scratch 

COPY --from=builder /usr/local/cargo/bin/bastid /
ENV PATH /
CMD ["/bastid"]
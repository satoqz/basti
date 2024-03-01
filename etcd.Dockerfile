FROM alpine:3.19 
RUN apk add --no-cache etcd etcd-ctl --repository=https://dl-cdn.alpinelinux.org/alpine/edge/testing
CMD ["etcd"]

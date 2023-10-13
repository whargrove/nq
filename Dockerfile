FROM rust:latest as builder
WORKDIR /usr/src/app
# TODO Optimize build time so that compiled dependencies are cached
COPY . .
RUN cargo build --release

FROM debian:12-slim
COPY --from=builder /usr/src/app/target/release/nq /usr/local/bin/nq
ENV BIND_ADDR=0.0.0.0
ENV PORT=3000
ENV HOSTNAME=nq.kentave.net
ENV RUST_LOG=info
EXPOSE ${PORT}
CMD ["nq"]

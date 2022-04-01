FROM rust:latest as builder

WORKDIR /usr/src/server
COPY . .

RUN cargo build --release && cargo doc

FROM debian:bullseye-slim
RUN apt-get update && apt-get install -y libgcc-10-dev && rm -rf /var/lib/apt/lists/
WORKDIR /usr/src/server
COPY --from=builder /usr/src/server/target/release/static-site-hosting /usr/local/bin/site
COPY --from=builder /usr/src/server/target/doc/ .

CMD ["/usr/local/bin/site"]

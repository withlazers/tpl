FROM clux/muslrust:1.60.0 AS build
WORKDIR /src
COPY . .
RUN cargo build --release

FROM alpine:3.16
COPY --from=build \
	/src/target/x86_64-unknown-linux-musl/release/tpl \
	/usr/local/bin

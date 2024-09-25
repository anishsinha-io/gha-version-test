FROM messense/cargo-zigbuild:sha-4e5d0f9 as builder

WORKDIR /app

COPY . .

RUN cargo zigbuild --release --target x86_64-unknown-linux-musl
# RUN cargo zigbuild --release --target aarch64-unknown-linux-musl

FROM scratch AS runtime
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/gha-version-test /usr/local/bin/
# COPY --from=builder /app/target/aarch64-unknown-linux-musl/release/scipio /usr/local/bin/

CMD ["/usr/local/bin/gha-version-test"]


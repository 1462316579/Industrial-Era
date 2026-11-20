# Built from the repository root: docker build -f crates/industrial-era-telemetry-server/Dockerfile .
FROM rust:1-slim AS build
WORKDIR /src
# The root manifest only for the shared crate's workspace settings (edition, lints).
COPY Cargo.toml Cargo.toml
COPY crates/industrial-era-telemetry crates/industrial-era-telemetry
COPY crates/industrial-era-telemetry-server crates/industrial-era-telemetry-server
RUN cargo build --release --locked --manifest-path crates/industrial-era-telemetry-server/Cargo.toml

FROM debian:stable-slim
COPY --from=build /src/crates/industrial-era-telemetry-server/target/release/industrial-era-telemetry-server /usr/local/bin/
ENV HORIZON_TELEMETRY_DB=/data/telemetry.sqlite3 HORIZON_TELEMETRY_TRUST_PROXY=1
CMD ["industrial-era-telemetry-server"]

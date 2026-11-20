# The Play with friends relay, for anyone to run (docs/play-with-friends.md, "Running your own relay").
# Build context: the repository root (the workspace's manifests and lockfile):
#   docker build -f crates/industrial-era-relay/Dockerfile -t industrial-era-relay .
FROM rust:1-slim-bookworm AS build
WORKDIR /src
COPY . .
RUN cargo build --release --locked -p industrial-era-relay \
    && cp target/release/industrial-era-relay /industrial-era-relay

FROM debian:bookworm-slim
COPY --from=build /industrial-era-relay /usr/local/bin/industrial-era-relay
ENV PORT=7000
EXPOSE 7000
USER nobody
CMD ["industrial-era-relay"]

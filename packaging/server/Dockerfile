# industrial-era-server in a container (docs/server.md, section 3). Build context: the repository root:
#   docker build -f packaging/server/Dockerfile -t industrial-era-server .
#
# The image holds none of Minecraft's files. Prepare them on the host from your own Minecraft 26.3
# (industrial-era-prepare --prepare, see packaging/PLAY.txt) and mount the folder it writes, read-only:
#   docker run -d --name industrial-era -p 25565:25565 --stop-timeout 180 \
#     -v industrial-era-data:/data \
#     -v /path/to/.local/industrial-era-client/26.3:/opt/industrial-era/.local/industrial-era-client/26.3:ro \
#     -e HORIZON_EULA=true -e HORIZON_WHITELIST=true -e HORIZON_PLAYERS=Alex,Steve -e HORIZON_OP=Alex \
#     industrial-era-server
#
# HORIZON_EULA=true accepts the Minecraft EULA (https://aka.ms/MinecraftEULA) for this server. The first
# start writes /data/server.toml from HORIZON_* (EULA, WORLD, SEED, PORT, WHITELIST, PLAYERS, OP); later
# starts use the file. Manage it with `docker exec industrial-era industrial-era-server status|cmd|backup|logs ...`.
FROM rust:1-slim-bookworm AS build
WORKDIR /src
COPY . .
# The server build of the game (no sound, no controllers: it needs neither ALSA nor udev) as industrial-era-engine.
RUN cargo build --release --locked -p industrial-era-client --no-default-features --bin industrial-era-client \
    && cargo build --release --locked -p industrial-era-server --bin industrial-era-server \
    && mkdir /out && cp target/release/industrial-era-client /out/industrial-era-engine && cp target/release/industrial-era-server /out/

FROM debian:bookworm-slim
RUN useradd --system --home-dir /data --shell /usr/sbin/nologin industrial-era \
    && mkdir -p /data /opt/industrial-era/.local/industrial-era-client \
    && chown industrial-era:industrial-era /data
COPY --from=build /out/ /opt/industrial-era/
ENV PATH=/opt/industrial-era:$PATH \
    HORIZON_NO_AUDIO=1
WORKDIR /data
VOLUME /data
EXPOSE 25565
USER industrial-era
# SIGTERM makes industrial-era-server stop the game cleanly (it saves first): give it --stop-timeout 180.
CMD ["sh", "-c", "test -f server.toml || industrial-era-server init --yes || exit 1; exec industrial-era-server run"]

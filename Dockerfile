# Multi-stage build for the persistent home server, `squire-serve` (SQUIRE-T-0129).
#
# The builder compiles the one binary in release mode; the runtime stage is a slim Debian with no
# toolchain. SQLite is bundled (libsqlite3-sys `bundled`), TLS is rustls, and the Keep's web assets
# are embedded at compile time (rust-embed), so the binary needs nothing from the OS beyond libc.

# --- build stage ---------------------------------------------------------
FROM rust:1-bookworm AS builder

WORKDIR /src
COPY . .

# Only the crate REGISTRY is cache-mounted (downloads; content-addressed, cannot go stale).
# The `target/` dir is deliberately NOT: cargo decides freshness by mtime, `COPY` preserves the
# host's mtimes, and a cached fingerprint newer than an edited source makes cargo skip the
# recompile — the image then ships the PREVIOUS binary with no error. That happened on this
# stack's first test (SQUIRE-T-0129): a security fix was in the source and absent from the image.
# A cold compile per build is the price of `redeploy` meaning what it says.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    cargo build --release --locked -p squire-home --bin squire-serve --bin reset_secret \
    && cp target/release/squire-serve target/release/reset_secret /usr/local/bin/

# --- runtime stage -------------------------------------------------------
FROM debian:bookworm-slim

# curl is the healthcheck probe; ca-certificates for the outbound APK sync (GitHub, HTTPS).
RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates \
        curl \
    && rm -rf /var/lib/apt/lists/* \
    # Explicit ids: the data volume's files are owned by these, so a base-image bump must not be
    # able to move them (`useradd --system` would pick whatever is free).
    && groupadd --system --gid 1000 squire \
    && useradd --system --uid 1000 --gid 1000 --create-home squire \
    # Pre-create the data dir owned by `squire`: Docker copies the image dir's owner onto a FRESH
    # named volume mounted here. Without it the volume is root-owned and uid 1000 cannot write the
    # signing key or the household database.
    && mkdir -p /data && chown squire:squire /data

COPY --from=builder /usr/local/bin/squire-serve /usr/local/bin/squire-serve
# The "forgot my password" escape hatch (`angreal deploy reset-secret`). It has to ship in the image:
# the household lives in a Docker volume, which a host-side binary cannot open.
COPY --from=builder /usr/local/bin/reset_secret /usr/local/bin/reset_secret

USER squire
WORKDIR /home/squire

# Container defaults — each is a behaviour that is wrong inside a container, not a preference:
#  * SQUIRE_SELF_UPDATE=off — the IMAGE is the unit of update. A self-replaced binary would vanish
#    on the next recreate (and uid 1000 cannot write /usr/local/bin anyway).
#  * SQUIRE_MDNS=off — multicast never leaves a bridge network (nor Docker Desktop's VM at all);
#    the host advertises instead, see deploy/advertise-mdns.sh.
# KEEP_BIND is deliberately NOT set here: it is only safe next to a port mapping that keeps the
# Keep operator-only, so it lives in deploy/docker-compose.yml beside that mapping.
ENV SQUIRE_DATA_DIR=/data \
    SQUIRE_SELF_UPDATE=off \
    SQUIRE_MDNS=off

VOLUME /data
EXPOSE 8080 4920

ENTRYPOINT ["/usr/local/bin/squire-serve"]

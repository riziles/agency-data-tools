# ── Build stage: just need the Rust binary ──
# (binary is pre-built on host; we copy it in)
# ── Runtime ──
FROM debian:trixie-slim

# Install Node.js 22 + curl (for health checks) + ca-certificates (for TLS)
RUN apt-get update && apt-get install -y --no-install-recommends \
    curl ca-certificates gnupg && \
    curl -fsSL https://deb.nodesource.com/setup_22.x | bash - && \
    apt-get install -y nodejs && \
    rm -rf /var/lib/apt/lists/*

# Install cloudflared (optional tunnel support)
RUN curl -sL https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-amd64 \
    -o /usr/local/bin/cloudflared && chmod +x /usr/local/bin/cloudflared

WORKDIR /app

# ── Copy app (data is bind-mounted at runtime) ──
COPY target/release/flight-sql-server /app/
COPY public/ /app/public/
COPY docker-entrypoint.sh /app/

# Seed directory structure for first run on an empty volume
RUN mkdir -p /app/data
VOLUME /app/data

RUN chmod +x /app/docker-entrypoint.sh

EXPOSE 8765

ENV APP_PASSWORD=demo
ENV TUNNEL=0

ENTRYPOINT ["/app/docker-entrypoint.sh"]

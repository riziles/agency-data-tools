# ── Runtime ──
FROM debian:trixie-slim

# Node.js 22 + curl (health checks) + ca-certificates (TLS)
RUN apt-get update && apt-get install -y --no-install-recommends \
    curl ca-certificates gnupg && \
    curl -fsSL https://deb.nodesource.com/setup_22.x | bash - && \
    apt-get install -y nodejs && \
    rm -rf /var/lib/apt/lists/*

# cloudflared for optional tunnel support
RUN curl -sL https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-amd64 \
    -o /usr/local/bin/cloudflared && chmod +x /usr/local/bin/cloudflared

WORKDIR /app

# ── Copy app ──
COPY flight-server/target/release/flight-sql-server /app/
COPY flight-server/server.mjs /app/
COPY flight-server/docker-entrypoint.sh /app/
COPY dashboard/build/ /app/dashboard/build/

# Seed directory structure for bind mount
RUN mkdir -p /app/data
VOLUME /app/data

RUN chmod +x /app/docker-entrypoint.sh

EXPOSE 8765 50051
# APP_PASSWORD default handled in docker-entrypoint.sh
ENV TUNNEL=0

ENTRYPOINT ["/app/docker-entrypoint.sh"]

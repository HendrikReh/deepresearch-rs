FROM python:3.11-slim-bullseye

ENV PYTHONDONTWRITEBYTECODE=1 \
    PYTHONUNBUFFERED=1 \
    MPLBACKEND=Agg \
    MPLCONFIGDIR=/tmp/matplotlib \
    PUPPETEER_EXECUTABLE_PATH=/usr/bin/chromium \
    PUPPETEER_SKIP_CHROMIUM_DOWNLOAD=true \
    PUPPETEER_ARGS="--no-sandbox,--disable-setuid-sandbox,--disable-dev-shm-usage" \
    DEBIAN_FRONTEND=noninteractive

RUN set -eux; \
    apt-get update; \
    apt-get install -y --no-install-recommends \
        build-essential \
        curl \
        git \
        graphviz \
        libgraphviz-dev \
        pkg-config \
        ca-certificates \
        fonts-liberation \
        libatk1.0-0 \
        libatk-bridge2.0-0 \
        libcairo2 \
        libdbus-1-3 \
        libexpat1 \
        libfontconfig1 \
        libfreetype6 \
        libgcc1 \
        libgdk-pixbuf2.0-0 \
        libglib2.0-0 \
        libgtk-3-0 \
        libnspr4 \
        libnss3 \
        libpango-1.0-0 \
        libpangocairo-1.0-0 \
        libx11-6 \
        libx11-xcb1 \
        libxcb1 \
        libxcomposite1 \
        libxcursor1 \
        libxdamage1 \
        libxext6 \
        libxfixes3 \
        libxi6 \
        libxrandr2 \
        libxrender1 \
        libxss1 \
        libxtst6 \
        unzip \
        xdg-utils; \
    rm -rf /var/lib/apt/lists/*

RUN set -eux; \
    curl -fsSL https://deb.nodesource.com/setup_20.x | bash -; \
    apt-get install -y --no-install-recommends nodejs chromium chromium-driver; \
    npm install -g @mermaid-js/mermaid-cli; \
    npm cache clean --force; \
    rm -rf /var/lib/apt/lists/*

RUN set -eux; \
    pip install --no-cache-dir \
        matplotlib \
        networkx \
        pandas \
        polars \
        pygraphviz \
        pydot

RUN set -eux; \
    useradd -ms /bin/bash sandbox; \
    mkdir -p /workspace; \
    chown sandbox:sandbox /workspace; \
    echo '{"args": ["--no-sandbox", "--disable-setuid-sandbox", "--disable-dev-shm-usage"]}' > /etc/mermaid-puppeteer.json; \
    mmdc_path=$(which mmdc); \
    mv "$mmdc_path" "${mmdc_path}.real"; \
    echo '#!/bin/sh' > "$mmdc_path"; \
    echo "exec ${mmdc_path}.real --puppeteerConfigFile /etc/mermaid-puppeteer.json \"\$@\"" >> "$mmdc_path"; \
    chmod +x "$mmdc_path"

USER sandbox
WORKDIR /workspace

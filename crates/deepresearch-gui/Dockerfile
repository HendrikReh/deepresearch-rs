# syntax=docker/dockerfile:1

FROM node:20-alpine AS frontend
WORKDIR /app
COPY crates/deepresearch-gui/web/package*.json ./
RUN npm ci
COPY crates/deepresearch-gui/web ./
RUN npm run build

FROM rust:1.75 AS builder
WORKDIR /app
COPY . .
COPY --from=frontend /app/dist crates/deepresearch-gui/web/dist
RUN cargo build --release -p deepresearch-gui

FROM debian:bullseye-slim
WORKDIR /opt/deepresearch
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/deepresearch-gui ./deepresearch-gui
EXPOSE 8080
ENV GUI_ENABLE_GUI=true
CMD ["./deepresearch-gui"]

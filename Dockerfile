# Image de production du site ORCHESTRA (Heroku, Render, Fly.io, Cloud Run…).
#
#   docker build -t orchestra . && docker run -p 8080:8080 orchestra

FROM rust:1.99-slim-trixie AS build
RUN apt-get update && apt-get install -y --no-install-recommends git ca-certificates \
    && rm -rf /var/lib/apt/lists/*
ENV CARGO_NET_GIT_FETCH_WITH_CLI=true
WORKDIR /app
COPY . .
RUN cargo build --release --locked --bin orchestra && cp target/release/orchestra /orchestra

FROM debian:trixie-slim
RUN useradd --system --uid 10001 app
WORKDIR /app
COPY --from=build /orchestra /usr/local/bin/orchestra
COPY public ./public
USER app
# Heroku fixe PORT au démarrage ; 8080 ailleurs.
ENV PORT=8080
EXPOSE 8080
CMD ["orchestra"]

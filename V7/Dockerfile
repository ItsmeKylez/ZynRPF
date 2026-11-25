# RPF Editor image (M4). Build: docker build -t rpf-editor .   Run: docker run -p 8080:8080 -v rpfdata:/data rpf-editor
# NOT BUILT OR RUN by the assistant (no Docker daemon or network in the build container). See DEPLOY_RAILWAY.md.
FROM node:22-slim AS web
WORKDIR /web
COPY web/package.json web/package-lock.json ./
RUN npm ci
COPY web/ ./
RUN npm run build

FROM rust:1-slim AS build
WORKDIR /src
# intel_tex_2 (texture encoder) links C++ code, so the build needs g++ (and libstdc++ at run time).
RUN apt-get update && apt-get install -y --no-install-recommends g++ && rm -rf /var/lib/apt/lists/*
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
RUN cargo build --release --locked -p server

FROM debian:stable-slim
RUN apt-get update && apt-get install -y --no-install-recommends libstdc++6 && rm -rf /var/lib/apt/lists/*
RUN useradd --no-create-home --shell /usr/sbin/nologin app && mkdir /data && chown app /data
COPY --from=build /src/target/release/server /usr/local/bin/server
COPY --from=web /web/dist /app/web
COPY docker-entrypoint.sh /usr/local/bin/docker-entrypoint.sh
# Git, zip files and Windows can drop the executable bit or add CRLF line endings, so fix both here.
RUN sed -i 's/\r$//' /usr/local/bin/docker-entrypoint.sh && chmod 755 /usr/local/bin/docker-entrypoint.sh
ENV DATA_DIR=/data COOKIE_SECURE=true WEB_DIR=/app/web
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/docker-entrypoint.sh"]
CMD ["server"]

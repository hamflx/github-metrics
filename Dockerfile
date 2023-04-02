FROM rust as builder

WORKDIR /usr/src/github-metrics
COPY . .

RUN cargo build --release

FROM debian
ENV GITHUB_DATA_DIR=/data

RUN sed -i 's/deb.debian.org/mirrors.ustc.edu.cn/g' /etc/apt/sources.list && apt-get update -y && apt-get install -y ca-certificates
WORKDIR /github-metrics
COPY --from=builder /usr/src/github-metrics/target/release/github-metrics /usr/bin/github-metrics
COPY web ./web

VOLUME ["/data"]
CMD ["/usr/bin/github-metrics"]

# ================ 第一阶段：编译 ================
FROM rust:1.92.0 AS builder

WORKDIR /app

# 安装 musl 工具链和 C 编译器
RUN rustup target add x86_64-unknown-linux-musl && \
    apt-get update && \
    apt-get install -y musl-tools musl-dev && \
    rm -rf /var/lib/apt/lists/*

# 先复制 Cargo 配置文件（利用 Docker 缓存）
COPY Cargo.toml Cargo.lock ./

# 创建一个空的 main.rs 来预编译依赖
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release --target x86_64-unknown-linux-musl && \
    rm -rf src

# 复制真正的源代码
COPY src ./src
COPY templates ./templates

# 使用 musl 目标编译
RUN cargo build --release --target x86_64-unknown-linux-musl

# ================ 第二阶段：运行时 ================
FROM scratch AS runtime

# 安装运行时依赖和 ca-certificates
#RUN apk add --no-cache ca-certificates

WORKDIR /app

# 创建非 root 用户
#RUN addgroup -S app && \
#    adduser -S app -G app

# 复制二进制文件（注意路径变化）
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/imail ./

# 设置文件权限
RUN chown -R app:app /app

# 切换到非 root 用户
USER app

# 暴露端口
EXPOSE 3000

CMD ["./imail"]

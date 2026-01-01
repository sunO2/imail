# ================ 第一阶段：编译 ================
# 使用 Rust Alpine 镜像构建静态链接二进制
FROM rust:1.92.0-alpine AS builder

# 安装构建依赖
RUN apk add --no-cache musl-dev pkgconfig

# 设置工作目录
WORKDIR /app

# 拷贝源代码
COPY . .

# 编译应用（静态链接到 musl）
RUN cargo build --release --target x86_64-unknown-linux-musl

# ================ 第二阶段：运行时 ================
# 使用 scratch 作为基础镜像（最小化，约 0MB）
# 如果需要 CA 证书，改用 alpine
FROM alpine:latest AS runtime

# 安装最小运行时依赖（CA 证书用于 HTTPS）
RUN apk add --no-cache ca-certificates

WORKDIR /app

# 从 builder 阶段拷贝编译好的二进制文件
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/imail ./

# 拷贝模板文件
COPY templates ./templates

# 设置非 root 用户以提高安全性
RUN addgroup -g 1000 appuser && \
    adduser -D -u 1000 -G appuser appuser
RUN chown -R appuser:appuser /app
USER appuser

# 设置容器启动时运行的命令
CMD ["./imail"]

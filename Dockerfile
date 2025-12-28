# ================ 第一阶段：编译 ================
# 使用官方 Rust 镜像作为 builder，包含 cargo、rustc 等完整工具链
FROM rust:1.92.0 AS builder

# 设置工作目录
WORKDIR /app

# 现在拷贝真正的源代码
COPY .config/config.toml ./.cargo/config.toml
COPY . .

RUN cargo check
# 编译真正的应用（--release 模式，优化并去掉调试信息）
# touch src/main.rs 是为了强制重新编译（因为之前用了假文件）
RUN cargo build --release

# ================ 第二阶段：运行时 ================
# 使用极小的 alpine Linux 作为基础镜像（只有几 MB）
# 如果你的应用需要 CA 证书或其他系统库，可以用 slim 版
FROM debian:bookworm-slim AS runtime

# 安装 HTTPS 需要的证书
RUN apt-get update && \
    apt-get install -y ca-certificates && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

# 从 builder 阶段拷贝编译好的二进制文件
COPY --from=builder /app/target/release/imail ./

# 设置容器启动时运行的命令
CMD ["/app/imail"]
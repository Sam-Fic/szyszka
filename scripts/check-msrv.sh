#!/usr/bin/env bash
# 本地复刻 CI 的 MSRV 闸门:读取 Cargo.toml 的 rust-version,
# 安装对应工具链并完整编译,确保声明的最低支持版本确实能编译通过。
# 用法: ./scripts/check-msrv.sh
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

MSRV=$(grep -m1 '^rust-version' Cargo.toml | sed -E 's/.*"([0-9.]+)".*/\1/')
echo "==> MSRV = $MSRV"

# 确保工具链已安装(已装则 rustup 会秒过)
if ! rustup toolchain list | grep -q "^$MSRV"; then
  echo "==> 安装 rustc $MSRV"
  rustup toolchain install "$MSRV" --profile minimal
fi

echo "==> cargo +$MSRV build --all-targets --all-features"
cargo +"$MSRV" build --all-targets --all-features
echo "✅ MSRV 编译通过"

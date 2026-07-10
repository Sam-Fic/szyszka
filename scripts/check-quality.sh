#!/usr/bin/env bash
# 本地复刻 GitHub Quality CI(quality.yml)，无需 Docker / act。
# 用法: ./scripts/check-quality.sh
set -euo pipefail

echo "==> cargo fmt --all -- --check"
cargo fmt --all -- --check

echo "==> cargo clippy --all-targets --all-features -- -D warnings"
cargo clippy --all-targets --all-features -- -D warnings

echo "==> cargo clippy --all-targets -- -D warnings"
cargo clippy --all-targets -- -D warnings

echo "✅ 全部通过，等价于 GitHub Quality CI"

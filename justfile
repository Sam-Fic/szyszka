run:
    cargo run

runr:
    cargo run --release

build:
    cargo build

buildr:
    cargo build --release

clip:
    cargo clippy --fix --allow-dirty --allow-staged --all-targets

# 本地复刻 GitHub Quality CI：fmt 检查 + clippy 严格门禁(-D warnings)
# 等价于 .github/workflows/quality.yml，无需 Docker / act，秒级反馈
quality:
    cargo fmt --all -- --check
    cargo clippy --all-targets --all-features -- -D warnings
    cargo clippy --all-targets -- -D warnings

# 先自动修复 clippy 再跑质量门禁；用于提交前自检
quality-fix:
    cargo clippy --fix --allow-dirty --allow-staged --all-targets --all-features
    cargo fmt --all
    just quality

# 本地复刻 CI 的 MSRV 闸门:用 Cargo.toml 声明的最低版本显式编译
msrv:
    MSRV=$$(grep -m1 '^rust-version' Cargo.toml | sed -E 's/.*"([0-9.]+)".*/\1/'); \
    rustup toolchain install "$$MSRV" --profile minimal; \
    cargo +"$$MSRV" build --all-targets --all-features

fix:
    grep -rlZ --include='*.rs' --include='*.slint' --include='*.md' --include='*.ftl' --exclude='AGENTS.md' --exclude='Justfile' '[─–—]' . | xargs -0 -r sed -i 's/[─–—]/-/g' || true
    cargo +nightly fmt
    cargo clippy --fix --allow-dirty --allow-staged --all-targets
    cargo +nightly fmt
    cargo fmt

upgrade:
    cargo update

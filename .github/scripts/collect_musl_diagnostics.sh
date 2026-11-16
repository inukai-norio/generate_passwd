#!/usr/bin/env bash
# SPDX-License-Identifier: ISC
# Copyright 2026 inukai-norio <inukay@proton.me>
# License text: TEST_CODE_LICENSE
# リリースと同じmuslターゲットをビルドし、リンク元の調査資料を保存する。
set -euo pipefail

if [[ $# -ne 2 ]]; then
    echo "Usage: $0 TARGET OUTPUT_DIR" >&2
    exit 2
fi

target=$1
output_dir=$2
case "$target" in
    x86_64-unknown-linux-musl|aarch64-unknown-linux-musl|i686-unknown-linux-musl) ;;
    *) echo "Unsupported musl target: $target" >&2; exit 2 ;;
esac

mkdir -p "$output_dir"
map_path="target/$target/release/musl-link.map"
binary_path="target/$target/release/generate_passwd"
# 以前の調査資料を今回のビルド結果として取り込まない。
test ! -e "$map_path"

{
    printf 'Target: %s\nCommit: ' "$target"
    git rev-parse HEAD
    rustc --version --verbose
    cargo --version --verbose
    rustup show active-toolchain
    cross --version
    docker version
} > "$output_dir/environment.txt" 2>&1

target_libdir=$(rustc --print target-libdir --target "$target")
if [[ -f "$target_libdir/self-contained/libc.a" ]]; then
    # Rust同梱版の照合用。実際の使用有無はリンカーマップで判断する。
    sha256sum "$target_libdir/self-contained/libc.a" \
        > "$output_dir/rust-sysroot-libc-sha256.txt"
fi

mkdir -p "target/$target/release"
if cross rustc -vv --locked --release --target "$target" --bin generate_passwd -- \
    -C "link-arg=-Wl,-Map=$map_path" 2>&1 | tee "$output_dir/build.log"; then
    build_status=0
else
    build_status=$?
fi
printf '%s\n' "$build_status" > "$output_dir/build-exit-code.txt"

# crossの詳細ログのイメージ名と、この一覧のID・digestを照合する。
# ビルドに失敗した場合にも、pullできたイメージの情報を残す。
docker image ls --no-trunc --digests > "$output_dir/docker-images.txt" 2>&1
if [[ -f "$map_path" ]]; then
    cp "$map_path" "$output_dir/musl-link.map"
fi
if [[ "$build_status" -ne 0 ]]; then
    exit "$build_status"
fi

test -s "$output_dir/musl-link.map"
test -s "$binary_path"
readelf --file-header --program-headers --dynamic "$binary_path" \
    > "$output_dir/elf-headers.txt"
sha256sum "$binary_path" > "$output_dir/binary-sha256.txt"
printf 'Saved musl diagnostics for %s to %s\n' "$target" "$output_dir"

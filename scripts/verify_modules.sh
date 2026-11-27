#!/usr/bin/env bash
# 递归查找含 Cargo.toml 的学习课，cargo run --manifest-path 执行。
# 用法:
#   ./scripts/verify_modules.sh 00_rust_basics
#   ./scripts/verify_modules.sh 00_rust_basics/00_hello
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

if ! command -v cargo >/dev/null 2>&1; then
  echo "❌ 需要安装 Rust（rustup）。安装: https://www.rust-lang.org/tools/install"
  echo "   学习入口: https://www.rust-lang.org/learn"
  exit 1
fi

echo "rustc: $(rustc --version 2>/dev/null || true)"
echo "cargo: $(cargo --version 2>/dev/null || true)"

if [[ $# -lt 1 ]]; then
  echo "用法: $0 <目录...>"
  exit 1
fi

total=0
passed=0
failed=0

run_one() {
  local manifest="$1"
  total=$((total + 1))
  echo "—— 运行: $manifest"
  if ! cargo run --manifest-path "$manifest" -q; then
    echo "❌ 运行失败: $manifest"
    failed=$((failed + 1))
    return
  fi
  passed=$((passed + 1))
}

for target in "$@"; do
  abs="$ROOT/$target"
  if [[ ! -e "$abs" ]]; then
    echo "❌ 不存在: $target"
    failed=$((failed + 1))
    continue
  fi
  echo ""
  echo "===== 验证目录: $target ====="
  while IFS= read -r -d '' file; do
    run_one "$file"
  done < <(find "$abs" -type f -name 'Cargo.toml' -print0 | sort -z)
done

echo ""
echo "全部汇总: 通过 $passed / $total ，失败 $failed"
if [[ "$failed" -gt 0 ]] || [[ "$total" -eq 0 ]]; then
  exit 1
fi

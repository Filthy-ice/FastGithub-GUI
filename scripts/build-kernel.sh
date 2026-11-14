#!/usr/bin/env bash
# 从官方源码编译 FastGithub 内核（retarget 到 .NET 8）。
#
# 为什么不用官方预编译二进制：FastGithub v2.1.5 官方包内置 .NET 7 运行时，
# 而 .NET 7 在 macOS 15 Sequoia+ 上因 AppleCrypto bug 无法生成自签证书
# （CreateSelfSigned/CopyWithPrivateKey 抛 "item no longer valid"），且 .NET 7
# 已停止支持、永不修复。该 bug 仅在 .NET 6.0.34/8.0.10/9.0 修复。故从官方源码
# retarget 到 .NET 8（LTS，已修复）自行编译，三平台一致。
#
# 源码锚点（供应链）：
#   repo   creazyboyone/FastGithub（原始作者 dotnetcore/FastGithub，MIT）
#   tag    v2.1.5
#   commit 20ef81111fecc9ebed68a1166aaeb815cdcec8c1
#   许可证 MIT，Copyright (c) 2021 老九（随内核保留 LICENSE）
#
# 用法：scripts/build-kernel.sh [rid]
#   rid 默认按宿主平台推断（osx-x64 / win-x64 / linux-x64）。
#   产物输出到 src-tauri/resources/kernel/（供 Tauri 打包）。
#
# 依赖：.NET 8 SDK（>=8.0.10，含 macOS 修复）。可用 DOTNET 指定 dotnet 路径，
#       默认 ~/.dotnet/dotnet，其次 PATH 上的 dotnet。
set -euo pipefail

REPO="creazyboyone/FastGithub"
TAG="v2.1.5"
COMMIT="20ef81111fecc9ebed68a1166aaeb815cdcec8c1"
VERSION="2.1.5"

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DEST="$ROOT/src-tauri/resources/kernel"
CACHE="$ROOT/.cache"
mkdir -p "$CACHE"

# --- 选择 rid ---
RID="${1:-}"
if [ -z "$RID" ]; then
  case "$(uname -s)" in
    Darwin) RID="osx-x64" ;;
    Linux)  RID="linux-x64" ;;
    MINGW*|MSYS*|CYGWIN*) RID="win-x64" ;;
    *) echo "无法推断宿主平台，请显式传入 rid" >&2; exit 1 ;;
  esac
fi
echo "目标 RID: $RID"

# --- 选择 dotnet ---
if [ -n "${DOTNET:-}" ]; then
  DOTNET_BIN="$DOTNET"
elif [ -x "$HOME/.dotnet/dotnet" ]; then
  DOTNET_BIN="$HOME/.dotnet/dotnet"
else
  DOTNET_BIN="$(command -v dotnet)"
fi
echo "dotnet: $DOTNET_BIN ($("$DOTNET_BIN" --version))"

# --- 获取源码 tarball（仅官方 GitHub codeload，禁止镜像）---
TARBALL="$CACHE/fastgithub-$TAG-src.tar.gz"
if [ ! -f "$TARBALL" ]; then
  echo "下载官方源码 $REPO $TAG ..."
  curl -sSL --http1.1 --retry 3 --max-time 900 -o "$TARBALL" \
    "https://codeload.github.com/$REPO/tar.gz/refs/tags/$TAG"
fi

# 固定 tarball 哈希（首次记录，之后校验）。用 python3 算 SHA256，三平台通用。
HASH_FILE="$CACHE/fastgithub-$TAG-src.sha256"
ACTUAL="$(python3 -c 'import hashlib,sys;print(hashlib.sha256(open(sys.argv[1],"rb").read()).hexdigest())' "$TARBALL")"
if [ -f "$HASH_FILE" ]; then
  EXPECTED="$(cat "$HASH_FILE")"
  if [ "$ACTUAL" != "$EXPECTED" ]; then
    echo "源码 tarball SHA256 不匹配！期望 $EXPECTED 实际 $ACTUAL" >&2
    exit 1
  fi
  echo "源码 tarball SHA256 校验通过"
else
  echo "$ACTUAL" > "$HASH_FILE"
  echo "首次下载，已固定源码哈希到 $HASH_FILE"
fi

# 校验 tag 指向的 commit（供应链锚点）
TAG_OBJ="$(curl -sSL --http1.1 "https://api.github.com/repos/$REPO/git/ref/tags/$TAG" \
  | python3 -c 'import sys,json;print(json.load(sys.stdin)["object"]["sha"])')"
TAG_COMMIT="$(curl -sSL --http1.1 "https://api.github.com/repos/$REPO/git/tags/$TAG_OBJ" \
  | python3 -c 'import sys,json;print(json.load(sys.stdin)["object"]["sha"])')"
if [ "$TAG_COMMIT" != "$COMMIT" ]; then
  echo "警告：tag $TAG 指向 commit $TAG_COMMIT，与锚点 $COMMIT 不一致" >&2
fi
echo "源码 commit: $TAG_COMMIT"

# --- 解压 ---
SRC="$CACHE/src-$TAG"
rm -rf "$SRC"
mkdir -p "$SRC"
tar -xzf "$TARBALL" -C "$SRC" --strip-components=1

# --- retarget net7.0 -> net8.0，升级 Hosting 包 ---
sed -i.bak 's|<TargetFramework>net7.0</TargetFramework>|<TargetFramework>net8.0</TargetFramework>|' "$SRC/Directory.Build.props"
sed -i.bak 's|Version="7.0.0-rc\*"|Version="8.0.0"|g' "$SRC/FastGithub/FastGithub.csproj"
rm -f "$SRC/Directory.Build.props.bak" "$SRC/FastGithub/FastGithub.csproj.bak"

# --- 编译（自包含单文件；InformationalVersion 必须含 '+' 以兼容 ProductionVersion.Parse）---
rm -rf "$DEST"
mkdir -p "$DEST"
echo "编译 $RID ..."
( cd "$SRC" && "$DOTNET_BIN" publish FastGithub/FastGithub.csproj \
    -c Release -r "$RID" --self-contained true \
    -p:PublishSingleFile=true -p:EnableCompressionInSingleFile=true \
    -p:InformationalVersion="$VERSION+source" \
    -o "$DEST" )

# dnscrypt-proxy 需要执行权限（osx/linux）
find "$DEST/dnscrypt-proxy" -type f -name 'dnscrypt-proxy*' -exec chmod +x {} \; 2>/dev/null || true

echo "内核已编译到 $DEST"
ls -la "$DEST/fastgithub"* 2>/dev/null || ls -la "$DEST"
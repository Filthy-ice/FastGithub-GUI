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

# 校验 tag 指向的 commit（best-effort 供应链 transparency：网络/限流失败不阻断构建。
# 硬锚点是上面固定的 tarball SHA256，已足以保证源码一致）。
TAG_COMMIT=""
for _attempt in 1 2 3; do
  TAG_OBJ="$(curl -sSL --http1.1 --retry 2 --max-time 60 \
    "https://api.github.com/repos/$REPO/git/ref/tags/$TAG" 2>/dev/null \
    | python3 -c 'import sys,json;print(json.load(sys.stdin).get("object",{}).get("sha",""))' 2>/dev/null)" || true
  if [ -n "$TAG_OBJ" ]; then
    TAG_COMMIT="$(curl -sSL --http1.1 --retry 2 --max-time 60 \
      "https://api.github.com/repos/$REPO/git/tags/$TAG_OBJ" 2>/dev/null \
      | python3 -c 'import sys,json;print(json.load(sys.stdin).get("object",{}).get("sha",""))' 2>/dev/null)" || true
  fi
  [ -n "$TAG_COMMIT" ] && break
  sleep 2
done
if [ -n "$TAG_COMMIT" ]; then
  if [ "$TAG_COMMIT" != "$COMMIT" ]; then
    echo "警告：tag $TAG 指向 commit $TAG_COMMIT，与锚点 $COMMIT 不一致" >&2
  fi
  echo "源码 commit: $TAG_COMMIT"
else
  echo "提示：在线校验 tag commit 失败（网络/限流），已跳过；tarball SHA256 仍为硬锚点" >&2
fi

# --- 解压 ---
SRC="$CACHE/src-$TAG"
rm -rf "$SRC"
mkdir -p "$SRC"
tar -xzf "$TARBALL" -C "$SRC" --strip-components=1

# --- retarget net7.0 -> net8.0，升级 Hosting 包 ---
sed -i.bak 's|<TargetFramework>net7.0</TargetFramework>|<TargetFramework>net8.0</TargetFramework>|' "$SRC/Directory.Build.props"
sed -i.bak 's|Version="7.0.0-rc\*"|Version="8.0.0"|g' "$SRC/FastGithub/FastGithub.csproj"
rm -f "$SRC/Directory.Build.props.bak" "$SRC/FastGithub/FastGithub.csproj.bak"

# --- Windows 免管理员补丁：改用与 macOS/Linux 一致的 HTTP 代理模式 ---
# 上游 Windows 默认 WinDivert 抓包(需管理员)+反向代理端口，exe 清单 requireAdministrator，
# 普通用户 GUI 拉起报 os error 740，且不监听本 GUI 期望的 38457。改为：清单 asInvoker、
# Windows 也 ListenHttpProxy(38457)、去掉 AddPacketIntercept。机器级证书安装失败上游已
# try/catch 仅告警，信任由 GUI 装到用户 Root 兜底（与 macOS 一致）。
sed -i.bak 's|level="requireAdministrator"|level="asInvoker"|g' "$SRC/FastGithub/app.manifest"
rm -f "$SRC/FastGithub/app.manifest.bak"

python3 - "$SRC/FastGithub/Startup.cs" <<'PY'
import re, sys
p = sys.argv[1]
s = open(p, encoding='utf-8-sig').read()
s = re.sub(
    r'if\s*\(OperatingSystem\.IsWindows\(\)\)\s*\{\s*kestrel\.ListenHttpsReverseProxy\(\);.*?else\s*\{\s*kestrel\.ListenHttpProxy\(\);\s*\}',
    'kestrel.ListenHttpProxy();', s, flags=re.DOTALL)
s = re.sub(
    r'\n[ \t]*if\s*\(OperatingSystem\.IsWindows\(\)\)\s*\{\s*services\.AddPacketIntercept\(\);\s*\}',
    '', s, flags=re.DOTALL)
assert 'ListenHttpsReverseProxy' not in s, 'Startup.cs 监听分支补丁未生效'
assert 'AddPacketIntercept' not in s, 'Startup.cs 抓包补丁未生效'
assert 'kestrel.ListenHttpProxy();' in s, '缺少 ListenHttpProxy'
open(p, 'w', encoding='utf-8').write(s)
print('Startup.cs 已补丁：Windows 改用 ListenHttpProxy，移除 AddPacketIntercept')
PY

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
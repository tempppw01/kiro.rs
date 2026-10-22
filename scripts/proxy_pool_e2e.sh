#!/bin/sh
# 代理节点池端到端验证：启动真实服务，检查 Admin API 的读取、保存、停用与鉴权
set -eu

cd /app

ADMIN_KEY="sk-admin-test"
BASE_URL="http://127.0.0.1:18990"
WORKDIR=/tmp/pool-e2e

rm -rf "$WORKDIR"
mkdir -p "$WORKDIR"

cat > "$WORKDIR/config.json" <<'JSON'
{
  "host": "127.0.0.1",
  "port": 18990,
  "apiKey": "sk-test-key",
  "adminApiKey": "sk-admin-test",
  "proxyPool": {
    "nodes": [
      { "url": "http://127.0.0.1:19101" },
      { "url": "http://127.0.0.1:19102", "username": "u", "password": "p" }
    ],
    "strategy": "roundRobin",
    "maxFailures": 3,
    "cooldownSecs": 60
  }
}
JSON

cat > "$WORKDIR/credentials.json" <<'JSON'
[]
JSON

/app/target/debug/kiro-rs --config "$WORKDIR/config.json" --credentials "$WORKDIR/credentials.json" \
  > "$WORKDIR/server.log" 2>&1 &
SERVER_PID=$!
trap 'kill "$SERVER_PID" 2>/dev/null || true' EXIT

ready=false
i=0
while [ "$i" -lt 60 ]; do
  if curl -fsS \
    -H "x-api-key: $ADMIN_KEY" \
    "$BASE_URL/api/admin/config/proxy-pool" >/dev/null 2>&1; then
    ready=true
    break
  fi
  sleep 0.5
  i=$((i + 1))
done

if [ "$ready" != true ]; then
  echo "服务启动失败" >&2
  cat "$WORKDIR/server.log" >&2
  exit 1
fi

request() {
  method=$1
  path=$2
  body=${3-}
  if [ -n "$body" ]; then
    curl -fsS -X "$method" \
      -H "Content-Type: application/json" \
      -H "x-api-key: $ADMIN_KEY" \
      -d "$body" \
      "$BASE_URL$path"
  else
    curl -fsS -X "$method" -H "x-api-key: $ADMIN_KEY" "$BASE_URL$path"
  fi
}

echo "== GET 初始代理节点池 =="
initial=$(request GET /api/admin/config/proxy-pool)
printf '%s\n' "$initial"
printf '%s' "$initial" | grep -q '"enabled":true'
printf '%s' "$initial" | grep -q '"total":2'
printf '%s' "$initial" | grep -q '"hasAuth":true'

echo ""
echo "== PUT 启用并保存手动填写的代理 =="
updated=$(request PUT /api/admin/config/proxy-pool '{
  "enabled": true,
  "proxyUrls": "# 手工填写\nhttp://127.0.0.1:19201\nsocks5://user:p%40ss@127.0.0.1:19202",
  "strategy": "random",
  "maxFailures": 4,
  "cooldownSecs": 90
}')
printf '%s\n' "$updated"
printf '%s' "$updated" | grep -q '"enabled":true'
printf '%s' "$updated" | grep -q '"strategy":"random"'
printf '%s' "$updated" | grep -q '"maxFailures":4'
printf '%s' "$updated" | grep -q '"cooldownSecs":90'
printf '%s' "$updated" | grep -q '"total":2'
printf '%s' "$updated" | grep -q '"hasAuth":true'
if printf '%s' "$updated" | grep -q 'user'; then
  echo "响应泄露了代理用户名" >&2
  exit 1
fi
if printf '%s' "$updated" | grep -q 'p%40ss'; then
  echo "响应泄露了代理密码" >&2
  exit 1
fi
grep -q '127.0.0.1:19202' "$WORKDIR/config.json"
grep -q '"strategy": "random"' "$WORKDIR/config.json"

echo ""
echo "== GET 保存后的代理节点池 =="
saved=$(request GET /api/admin/config/proxy-pool)
printf '%s\n' "$saved"
printf '%s' "$saved" | grep -q '"strategy":"random"'
printf '%s' "$saved" | grep -q '"total":2'

echo ""
echo "== PUT 停用代理节点池 =="
disabled=$(request PUT /api/admin/config/proxy-pool '{
  "enabled": false,
  "proxyUrls": "",
  "strategy": "roundRobin",
  "maxFailures": 3,
  "cooldownSecs": 60
}')
printf '%s\n' "$disabled"
printf '%s' "$disabled" | grep -q '"enabled":false'
printf '%s' "$disabled" | grep -q '"total":0'

echo ""
echo "== GET 停用后的代理节点池 =="
after_disable=$(request GET /api/admin/config/proxy-pool)
printf '%s\n' "$after_disable"
printf '%s' "$after_disable" | grep -q '"enabled":false'
printf '%s' "$after_disable" | grep -q '"total":0'

echo ""
echo "== POST 重置代理节点池状态 =="
request POST /api/admin/config/proxy-pool/reset
echo ""

echo ""
echo "== 未认证访问（应返回 401） =="
status=$(curl -s -o /dev/null -w "%{http_code}" "$BASE_URL/api/admin/config/proxy-pool")
printf 'status=%s\n' "$status"
test "$status" = "401"

echo ""
echo "代理节点池 E2E 验证通过"

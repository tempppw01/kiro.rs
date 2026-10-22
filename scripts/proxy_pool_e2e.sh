#!/bin/sh
# 代理节点池端到端验证：启动真实服务，检查 Admin API 的节点池状态与重置
set -e

cd /app

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
trap 'kill $SERVER_PID 2>/dev/null || true' EXIT

# 等待服务启动
i=0
while [ $i -lt 40 ]; do
  if curl -s -o /dev/null "http://127.0.0.1:18990/health" 2>/dev/null; then break; fi
  if curl -s -o /dev/null -H "x-api-key: sk-admin-test" "http://127.0.0.1:18990/api/admin/config/proxy-pool" 2>/dev/null; then break; fi
  sleep 0.5
  i=$((i + 1))
done

echo "== 服务启动日志 =="
grep -E "代理节点池|代理" "$WORKDIR/server.log" || cat "$WORKDIR/server.log"

echo ""
echo "== GET /api/admin/config/proxy-pool =="
curl -s -H "x-api-key: sk-admin-test" "http://127.0.0.1:18990/api/admin/config/proxy-pool"
echo ""

echo ""
echo "== POST /api/admin/config/proxy-pool/reset =="
curl -s -X POST -H "x-api-key: sk-admin-test" "http://127.0.0.1:18990/api/admin/config/proxy-pool/reset"
echo ""

echo ""
echo "== 未认证访问（应返回 401） =="
curl -s -o /dev/null -w "status=%{http_code}\n" "http://127.0.0.1:18990/api/admin/config/proxy-pool"

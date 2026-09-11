#!/usr/bin/env bash
set -euo pipefail
image=${1:?Provide an image tag}
name=etyloom-smoke-$$
volume=$name-data
cleanup() {
  docker logs "$name" > container.log 2>&1 || true
  docker rm -f "$name" >/dev/null 2>&1 || true
  docker volume rm "$volume" >/dev/null 2>&1 || true
}
trap cleanup EXIT
if docker run --rm -e BASE_URL=http://insecure.invalid "$image"; then
  echo 'Production accepted insecure configuration' >&2
  exit 1
fi
docker volume create "$volume" >/dev/null
docker run -d --name "$name" --read-only --cap-drop ALL --security-opt no-new-privileges \
  --tmpfs /tmp:size=64m --mount "source=$volume,target=/data" -p 127.0.0.1:18080:3000 \
  -e BASE_URL=https://etyloom.invalid -e OIDC_ISSUER_URL=https://identity.invalid \
  -e OIDC_CLIENT_ID=container-smoke -e OIDC_CLIENT_SECRET=not-a-live-secret "$image"
for attempt in $(seq 1 60); do
  if curl --fail --silent http://127.0.0.1:18080/health/ready >/dev/null; then break; fi
  sleep 1
done
curl --fail --silent http://127.0.0.1:18080/health/ready >/dev/null
curl --fail --silent http://127.0.0.1:18080/ > container-index.html
grep -q 'Etyloom' container-index.html
curl --fail --silent http://127.0.0.1:18080/boot.js > container-boot.js
grep -q '/pkg/etyloom.wasm' container-boot.js
curl --fail --silent -D container-wasm.headers http://127.0.0.1:18080/pkg/etyloom.wasm -o container.wasm
grep -qi 'content-type: application/wasm' container-wasm.headers
test "$(od -An -tx1 -N4 container.wasm | tr -d ' \n')" = 0061736d
for route in /api/projects /api/languages /api/jobs; do
  test "$(curl --silent -o /dev/null -w '%{http_code}' "http://127.0.0.1:18080$route")" = 401
done
test "$(curl --silent -o /dev/null -w '%{http_code}' http://127.0.0.1:18080/auth/dev)" = 404
test "$(docker exec "$name" id -u)" = 10001
docker exec "$name" sh -c '! command -v python && ! command -v python3'
docker exec "$name" etyloom-web backup /data/smoke-backup.db
docker restart "$name" >/dev/null
for attempt in $(seq 1 30); do
  if docker exec "$name" etyloom-web healthcheck; then break; fi
  sleep 1
done
docker exec "$name" etyloom-web healthcheck
docker exec "$name" test -s /data/smoke-backup.db
echo 'Container smoke checks passed'

# ContextForge Production Deployment Guide

Production-ready deployment for the ContextForge daemon (Rust data-plane core +
Go Console contract REST proxy). Targets the **two-process layout** documented in
[ADR-016 D3](../decisions/adr-016-cross-process-rust-go-via-grpc-bridge.md) (Rust
SoT + Go thin proxy) with **fallback deny** by default per [ADR-018](../decisions/adr-018-fallback-inmem-default-reversal.md).

The reference stack is `deploy/docker-compose.production.yml` (shipped task-16.4,
Phase 16). For development / PoC, use `deploy/console-stack.yml` instead (single
container, fallback in-mem enabled).

---

## §1 Quick start

```bash
# 1. Pull image (task-16.3 ships ghcr.io/tajiaoyezi/contextforge-daemon)
docker pull ghcr.io/tajiaoyezi/contextforge-daemon:v0.28.0

# 2. Bring up the stack (uses defaults from compose yml — see §2 for overrides)
docker compose -f deploy/docker-compose.production.yml up -d

# 3. Wait for both services to report healthy (~10–30 seconds)
docker compose -f deploy/docker-compose.production.yml ps

# 4. Verify
curl -fsS http://localhost:48181/v1/health | jq .
# Expect: {"status":"healthy","contract_version":"v1",...}
```

If `/v1/health` returns 503 or `status: "degraded"`, the Rust `contextforge-core`
daemon is unreachable — see §8 troubleshooting.

---

## §2 Image source + version pinning

The compose stack reads two env vars to build the image reference:

```yaml
image: ghcr.io/${OWNER:-tajiaoyezi}/contextforge-daemon:${CONTEXTFORGE_VERSION:-v0.28.0}
```

To pin a specific version, copy and edit the env template:

```bash
cp deploy/.env.production.example deploy/.env.production
# edit deploy/.env.production:
#   OWNER=tajiaoyezi
#   CONTEXTFORGE_VERSION=v0.28.0

docker compose --env-file deploy/.env.production \
  -f deploy/docker-compose.production.yml up -d
```

Available tags:
- `:v0.28.0` — current stable release (always pin an explicit `vX.Y.Z` in production)
- `:latest` — moves to the latest published `v*` tag (avoid in production pins)
- `:v0.9.0` etc. — earlier stable releases remain pullable for downgrade

Images are built by `.github/workflows/release.yml` (task-16.3) on every `v*`
annotated tag push and pushed to GHCR as `linux/amd64` only. ARM64 support is
tracked under [SPEC-DEFER:phase-future.multi-arch-image].

**Supply-chain verification** (cosign keyless, task-28.2) — the release image is
signed and carries an SPDX SBOM + SLSA provenance attestation. Verify the exact
digest before deploying:

```bash
# resolve the digest the tag points to, then verify that digest
DIGEST=$(docker buildx imagetools inspect \
  ghcr.io/tajiaoyezi/contextforge-daemon:v0.28.0 --format '{{.Manifest.Digest}}')
cosign verify ghcr.io/tajiaoyezi/contextforge-daemon@"$DIGEST" \
  --certificate-identity-regexp '^https://github.com/tajiaoyezi/contextforge/.github/workflows/release.yml@.*$' \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com
```

The per-release digest + Rekor transparency-log indices are recorded in
`docs/releases/v0.28.0-evidence.md` (and the matching `RELEASE_NOTES.md` entry).

---

## §3 Data persistence

State lives in the `contextforge-data` named volume, mounted at `/data` inside
both containers. Contents include:

| Path | Contents | Owner |
|---|---|---|
| `/data/workspaces.db` | SQLite — workspace metadata | task-11.3 |
| `/data/index-jobs.db` | SQLite — index job state | Phase 10 |
| `/data/memory.db` | SQLite — memory items | task-13.x |
| `/data/eval-runs.db` | SQLite — eval run history | task-14.x |
| `/data/search-traces.db` | SQLite — search query history (task-16.1) | Phase 16 |
| `/data/index/` | Tantivy index files | core/data_plane |

**Backup (volume export)**:

```bash
# Snapshot to a tarball — daemon can keep running, but for consistency stop first
docker compose -f deploy/docker-compose.production.yml stop
docker run --rm \
  -v contextforge_contextforge-data:/data \
  -v $(pwd):/backup \
  alpine tar czf /backup/contextforge-data-$(date +%F).tgz -C /data .
docker compose -f deploy/docker-compose.production.yml start
```

**Restore**:

```bash
docker compose -f deploy/docker-compose.production.yml down
docker volume create contextforge_contextforge-data
docker run --rm \
  -v contextforge_contextforge-data:/data \
  -v $(pwd):/backup \
  alpine tar xzf /backup/contextforge-data-YYYY-MM-DD.tgz -C /data
docker compose -f deploy/docker-compose.production.yml up -d
```

**Wipe (data loss — irreversible)**:

```bash
docker compose -f deploy/docker-compose.production.yml down -v
```

---

## §4 Health check semantics

`GET /v1/health` returns one of three states (task-15.6):

| Status | Meaning | Action |
|---|---|---|
| `healthy` | Both Rust core gRPC + REST proxy responsive; data-plane reachable | None |
| `degraded` | REST proxy up, but core unreachable AND fallback in-mem allowed (legacy mode only) | Investigate core |
| 503 + `unreachable` | REST proxy up, core unreachable, fallback deny (ADR-018 default) | Restart core; check logs |

Production stack (this compose) uses **fallback deny** — see [ADR-018](../decisions/adr-018-fallback-inmem-default-reversal.md).
You should **never see `degraded`** here; that mode is reserved for the dev/PoC
`console-stack.yml` which sets `CONSOLE_API_FALLBACK_INMEM=1`.

The docker healthcheck (`curl -fsS /v1/health`) treats any non-2xx response as
unhealthy. Container restart policy is `restart: unless-stopped` — manual stops
stay stopped; crash / OOM trigger restart.

For richer telemetry, `GET /v1/health?detailed=true` returns per-component
status (db / index / embed / retriever / eval — task-15.6).

**Wildcard bind opt-in**: the Rust core refuses `0.0.0.0` by default (dev
safety baseline — `core::server::resolve_listen_addr`). For docker / k8s
deployment where container network isolation makes wildcard bind safe, the
opt-in env var `CONTEXTFORGE_ALLOW_WILDCARD_BIND=1` allows it. The compose
stack sets this on `contextforge-core` because the daemon needs `0.0.0.0:50551`
to be reachable across the docker bridge network. Port 50551 is **not** mapped
to the host — only the `contextforge-core` service container IP is reachable
from `console-api-serve`.

---

## §5 Auth

Both `trusted-network` and `bearer-token` modes are supported. Default is
trusted-network (any caller on the docker bridge can call `/v1/*`).

To enforce token auth, set in `.env.production`:

```bash
CONSOLE_API_AUTH_TOKEN=<random-32-byte-hex>
```

This is passed through to the `console-api-serve` container as env
`CONTEXTFORGE_CONSOLEAPI_AUTH_TOKEN`. The Go REST proxy then requires:

```
Authorization: Bearer <token>
```

on every `/v1/*` route. Unauthorized requests get 401.

Trusted-network mode (empty token) is appropriate when the stack is fronted by
a reverse proxy (nginx / traefik / caddy) that already enforces auth, or when
ContextForge is reachable only from a private VPC.

**Optional TLS termination** (task-31.2 / ADR-036 D2 — landed): the production
compose ships an optional Caddy reverse proxy under the `tls` profile, NOT started
by default (the existing plaintext deployment is unchanged):

```bash
docker compose -f deploy/docker-compose.production.yml --profile tls up -d
```

It terminates TLS on 443 and reverse-proxies to `console-api-serve:48181`. Edit
`deploy/caddy/Caddyfile` with your domain. With a real domain + reachable :80/:443,
Caddy auto-provisions a Let's Encrypt cert (ACME) — that real cert issuance needs a
live domain and is deferred (`[SPEC-DEFER:phase-future.compose-tls-auto-cert]`); for
internal deployments use `tls internal` (Caddy local CA) or mount your own cert.

---

## §6 Upgrade path

Upgrading to a newer tag preserves data (example targets v0.28.0):

```bash
# 1. Pull new image
docker pull ghcr.io/tajiaoyezi/contextforge-daemon:v0.28.0

# 2. Update .env.production (optional — only if pinning explicitly)
sed -i 's/CONTEXTFORGE_VERSION=.*/CONTEXTFORGE_VERSION=v0.28.0/' deploy/.env.production

# 3. Recreate containers, keep volume
docker compose -f deploy/docker-compose.production.yml up -d --force-recreate
```

The `contextforge-data` volume is **not** removed — `up -d --force-recreate`
only stops and recreates the containers. SQLite migrations (e.g.
`0015_search_traces.sql` — task-16.1, or later `0019_indexing_events` — task-33.3)
run automatically on first start of the new image; they are `IF NOT EXISTS` /
guarded `ALTER` and idempotent.

To downgrade, point `CONTEXTFORGE_VERSION` back to a prior tag and recreate.
**Caveat**: a tag containing a forward-migrating schema change (e.g. v0.9.0's
`search_traces` table) leaves the schema in place under an older binary, which
is harmless but unused.

---

## §7 Kubernetes equivalent (skeleton)

For users on Kubernetes, here's the manifest skeleton equivalent of the
compose stack. A full Helm chart with values is deferred under
[SPEC-DEFER:phase-future.k8s-helm-chart] (v1.x).

```yaml
# k8s/contextforge.yaml (skeleton — adapt for your cluster)

apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: contextforge-data
spec:
  accessModes: ["ReadWriteOnce"]
  resources:
    requests:
      storage: 10Gi

---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: contextforge
spec:
  replicas: 1   # single-writer; shared volume forbids replicas: > 1
  selector:
    matchLabels:
      app: contextforge
  template:
    metadata:
      labels:
        app: contextforge
    spec:
      containers:
      - name: contextforge-core
        image: ghcr.io/tajiaoyezi/contextforge-daemon:v0.28.0
        # Same pod = shared network namespace; console-api-serve dials
        # 127.0.0.1:50551 so wildcard bind is unnecessary here (unlike compose).
        command: ["contextforge-core", "127.0.0.1:50551", "/data"]
        env:
        - name: CONTEXTFORGE_DATA_DIR
          value: /data
        - name: RUST_LOG
          value: info
        volumeMounts:
        - name: data
          mountPath: /data
        readinessProbe:
          tcpSocket:
            port: 50551
          initialDelaySeconds: 10
          periodSeconds: 10

      - name: console-api-serve
        image: ghcr.io/tajiaoyezi/contextforge-daemon:v0.28.0
        command:
        - contextforge
        - console-api-serve
        - --addr
        - 0.0.0.0:48181
        - --grpc-addr
        - 127.0.0.1:50551     # same pod; localhost
        env:
        - name: CONTEXTFORGE_CONSOLEAPI_AUTH_TOKEN
          valueFrom:
            secretKeyRef:
              name: contextforge-auth
              key: token
              optional: true
        ports:
        - containerPort: 48181
          name: rest
        readinessProbe:
          httpGet:
            path: /v1/health
            port: 48181
          initialDelaySeconds: 5
          periodSeconds: 5

      volumes:
      - name: data
        persistentVolumeClaim:
          claimName: contextforge-data

---
apiVersion: v1
kind: Service
metadata:
  name: contextforge
spec:
  selector:
    app: contextforge
  ports:
  - port: 48181
    targetPort: 48181
    name: rest
```

Note the K8s skeleton runs **both processes in the same pod** so `--grpc-addr`
can point to `127.0.0.1:50551` (no cross-pod DNS), whereas the compose stack
runs them in separate containers and uses the `contextforge-core` service name.
Either layout is supported.

---

## §8 Troubleshooting

### Both containers running but `/v1/health` returns 503

```bash
docker compose -f deploy/docker-compose.production.yml logs --tail 100 contextforge-core
docker compose -f deploy/docker-compose.production.yml logs --tail 100 console-api-serve
```

Look for in `contextforge-core` logs:
- `address already in use` → port 50551 collision (unlikely since not exposed to host)
- `Permission denied (os error 13)` → volume mount UID mismatch — check `chown` inside the volume

Look for in `console-api-serve` logs:
- `dial gRPC: context deadline exceeded` → DNS / connectivity to `contextforge-core:50551`
- `fallback denied` → expected when core is down (ADR-018); fix the core, not the proxy

### `depends_on: service_healthy` blocks `console-api-serve` startup

If `contextforge-core` takes longer than 60s to become healthy (start_period 10s
+ 6 retries × 10s interval), the dependency wait times out. Increase
`start_period` in the compose yml:

```yaml
  contextforge-core:
    healthcheck:
      start_period: 30s   # extend from 10s if slow disk / cold cache
```

### Slow `docker pull`

GHCR proxies through GitHub's CDN — typical pull is ~50 MB and < 1 min on a
warm cache. For air-gapped environments, `docker save` the image on a connected
host:

```bash
# on connected host
docker save ghcr.io/tajiaoyezi/contextforge-daemon:v0.28.0 | gzip > contextforge-v0.28.0.tar.gz
# transfer to air-gapped host
docker load < contextforge-v0.28.0.tar.gz
```

### Container restart loop

```bash
docker compose -f deploy/docker-compose.production.yml ps
# State: Restarting → check exit code via:
docker inspect <container-id> | jq '.[0].State'
```

Common root causes:
- OOM kill (exit 137) → bump memory limit (compose stack doesn't set one by default)
- SQLite lock contention on slow disk → check `iostat -x`; consider `volume.driver_opts: { type: ext4 }` or migrate to dedicated SSD

---

## §9 Performance tuning

**RUST_LOG**: default `info`. Set `RUST_LOG=warn` in `.env.production` to reduce
log volume on a busy daemon (~30% CPU savings on high-throughput workloads).
Set `RUST_LOG=debug` only when actively debugging — debug logs include per-query
trace dumps which inflate disk usage 10×.

**Volume mount type**: the default `local` driver uses the docker volume root
(usually `/var/lib/docker/volumes/...`). For better performance:

```yaml
volumes:
  contextforge-data:
    driver: local
    driver_opts:
      type: none
      o: bind
      device: /mnt/nvme/contextforge-data
```

This binds the volume to a dedicated NVMe path, bypassing the overlay filesystem
write-amplification.

**Container resource limits** (task-31.2 / ADR-036 D2 — landed): the production
compose now sets `mem_limit` / `cpus` on both services (compose v2 top-level keys —
`deploy.resources` only applies under swarm), env-overridable:

```yaml
  contextforge-core:
    mem_limit: ${CORE_MEM_LIMIT:-2g}
    cpus: ${CORE_CPUS:-1.5}
  console-api-serve:
    mem_limit: ${CONSOLE_MEM_LIMIT:-512m}
    cpus: ${CONSOLE_CPUS:-1.0}
```

Override per host, e.g. `CORE_MEM_LIMIT=4g CORE_CPUS=3.0 docker compose -f
deploy/docker-compose.production.yml up -d`.

**Concurrent query throughput**: ContextForge's data-plane processes search
queries serially per index (Tantivy reader pool is single-writer). For higher
concurrent throughput, deploy multiple ContextForge stacks behind a load-balanced
proxy keyed by workspace_id (HTTP path prefix matching) — out of scope for v0.9.

---

## See also

- [ADR-016: Cross-process Rust ↔ Go via gRPC bridge](../decisions/adr-016-cross-process-rust-go-via-grpc-bridge.md) — D3 two-process rationale
- [ADR-017: Console contract completion (22 endpoint)](../decisions/adr-017-console-contract-completion-22-endpoint.md) — REST surface
- [ADR-018: Fallback in-mem default reversal](../decisions/adr-018-fallback-inmem-default-reversal.md) — production fallback deny
- `deploy/console-stack.yml` — dev/PoC compose (single container, fallback enabled) — do **not** use in production
- `scripts/console_smoke.sh` — 27-step smoke (Phase 16 v7) including `COMPOSE_PROD_SMOKE=1` gated stack health check

# Poly VPN Architecture

**Version**: 3.0
**Date**: February 2026
**Platform**: eStream v0.9.1
**Upstream**: PolyKit v0.3.0, eStream graph/DAG constructs
**Build Pipeline**: FastLang (.fl) → FLIR → Rust/WASM codegen → .escd

---

## Overview

Poly VPN is a post-quantum encrypted, scatter-routed VPN. Traffic is not funneled through a single tunnel to a single exit node — it is scatter-routed across multiple exit nodes simultaneously so that no single exit sees the complete picture. Traffic mimicry (trade secret) makes VPN traffic indistinguishable from normal browsing. All cryptographic operations run in WASM (Rust). TypeScript is a DOM binding layer only.

### What Changed in v3.0

| Area | v2.0 | v3.0 |
|------|------|------|
| Exit mesh | Custom topology | `graph vpn_exit_mesh` (mirrors `device_mesh.fl`) |
| Routing | Custom scatter router | `dag tunnel_route` with typed overlays |
| Exit selection | VRF random | `ai_feed exit_selection` (optimal multi-exit route) |
| Circuit format | FLIR YAML (`circuit.flir.yaml`) | FastLang `.fl` with PolyKit profiles |
| RBAC | Per-circuit annotations | eStream `rbac.fl` composed via PolyKit |
| Platform | eStream v0.8.1 | eStream v0.9.1 |

---

## Zero-Linkage Privacy

Poly VPN operates under the Poly Labs zero-linkage privacy architecture:

- **HKDF context**: `poly-vpn-v1` — produces `user_id`, signing key, and encryption key that cannot be correlated with any other Poly product
- **Lex namespace**: `esn/global/org/polylabs/vpn` — completely isolated from other product namespaces
- **StreamSight**: Telemetry stays within `polylabs.vpn.*` lex paths
- **Metering**: Own `metering_graph` instance under `polylabs.vpn.metering` lex
- **Billing**: Tier checked via blinded token status, not cross-product identity

---

## Identity & Authentication

### SPARK Derivation Context

```
SPARK biometric → Secure Enclave/TEE → master_seed (in WASM, never exposed to JS)
                                            │
                                            ▼
                                   HKDF-SHA3-256(master_seed, "poly-vpn-v1")
                                            │
                                            ├── ML-DSA-87 signing key pair
                                            │   (tunnel setup, profile changes, exit attestation)
                                            │
                                            └── ML-KEM-1024 encryption key pair
                                                (per-exit session key exchange)
```

### User Identity

```
user_id = SHA3-256(spark_ml_dsa_87_public_key)[0..16]   # 16-byte truncated hash
```

All stream topics, tunnel ownership, and profiles reference this SPARK-derived `user_id`. There are no usernames, emails, or phone numbers. This `user_id` is unique to Poly VPN and cannot be linked to identities in other Poly products.

---

## Core Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                        Poly VPN Client                                │
│                                                                       │
│  ┌────────────────────────────────────────────────────────────────┐  │
│  │  UI Layer (Tauri desktop / React Native mobile)                  │  │
│  │  Dashboard │ Profiles │ Split Tunnel │ Net Shield │ Settings     │  │
│  └────────────────────────────┬───────────────────────────────────┘  │
│                                │                                      │
│  ┌────────────────────────────┴───────────────────────────────────┐  │
│  │  Graph/DAG Layer (WASM, backed by scatter-cas)                   │  │
│  │                                                                   │  │
│  │  graph vpn_exit_mesh    — exit node topology + overlays          │  │
│  │  dag tunnel_route       — multi-hop scatter routing DAG          │  │
│  │  graph metering_graph   — per-product 8D metering (PolyKit)     │  │
│  │  graph user_graph       — per-product identity (PolyKit)        │  │
│  └────────────────────────────┬───────────────────────────────────┘  │
│                                │                                      │
│  ┌────────────────────────────┴───────────────────────────────────┐  │
│  │  FastLang Circuits (WASM via .escd)                              │  │
│  │                                                                   │  │
│  │  polyvpn_encrypt.fl │ polyvpn_scatter.fl │ polyvpn_mimicry.fl   │  │
│  │  polyvpn_killswitch.fl │ polyvpn_dns.fl │ polyvpn_metering.fl  │  │
│  │  (all ML-DSA-87 signed .escd packages, StreamSight-annotated)   │  │
│  └────────────────────────────┬───────────────────────────────────┘  │
│                                │                                      │
│  ┌────────────────────────────┴───────────────────────────────────┐  │
│  │  poly-vpn-core (Rust)                                            │  │
│  │  TUN/TAP │ PQ Encrypt │ Scatter Router │ Kill Switch │ Mimicry  │  │
│  └────────────────────────────┬───────────────────────────────────┘  │
│                                │                                      │
│  ┌────────────────────────────┴───────────────────────────────────┐  │
│  │  eStream SDK (@estream/sdk-browser or react-native)              │  │
│  │  Wire protocol only: UDP :5000 / WebTransport :4433             │  │
│  └────────────────────────────┬───────────────────────────────────┘  │
└────────────────────────────────┼────────────────────────────────────────┘
                                 │
                        PQ Encrypt + Scatter
                                 │
                 ┌───────────────┼───────────────┐
                 │               │               │
           ┌─────┴─────┐  ┌─────┴─────┐  ┌─────┴─────┐
           │ Exit Node │  │ Exit Node │  │ Exit Node │
           │ A (US)    │  │ B (DE)    │  │ C (SG)    │
           │ mimicry:  │  │ mimicry:  │  │ mimicry:  │
           │ Netflix   │  │ YouTube   │  │ iCloud    │
           └─────┬─────┘  └─────┬─────┘  └─────┬─────┘
                 │               │               │
                 v               v               v
              Internet        Internet        Internet
```

---

## Graph/DAG Constructs

### VPN Exit Mesh (`polyvpn_exit_graph.fl`)

The exit node network is modeled as a typed graph, mirroring the eStream `device_mesh.fl` pattern. Exit nodes are nodes; tunnel connections are edges. Overlays provide real-time state (latency, bandwidth, load, jurisdiction, blacklist status) without mutating the base graph. The `ai_feed` selects optimal multi-exit scatter routes.

```fastlang
type ExitNode = struct {
    exit_id: bytes(16),
    region: bytes(8),
    capacity_mbps: u32,
    pubkey: bytes(1568),
    jurisdiction: bytes(4),
    operator_type: u8,
    created_at: u64,
}

type TunnelEdge = struct {
    hop_index: u8,
    encryption_layer: u8,
    session_key_hash: bytes(32),
    established_at: u64,
}

graph vpn_exit_mesh {
    node ExitNode
    edge TunnelEdge

    overlay latency_ns: u64 bitmask delta_curate
    overlay bandwidth_mbps: u32 bitmask delta_curate
    overlay load_pct: u8 curate delta_curate
    overlay jurisdiction: bytes(4) curate
    overlay blacklist_status: u8 curate delta_curate

    storage csr {
        hot @bram,
        warm @ddr,
        cold @nvme,
    }

    ai_feed exit_selection

    observe vpn_exit_mesh: [latency_ns, load_pct, bandwidth_mbps, blacklist_status] threshold: {
        anomaly_score 0.8
        baseline_window 30
    }
}

series exit_series: vpn_exit_mesh
    merkle_chain true
    lattice_imprint true
    witness_attest true
```

Key circuits: `register_exit`, `select_exits`, `rotate_exits`, `update_exit_health`, `blacklist_exit`, `attest_exit`.

### Tunnel Route DAG (`polyvpn_route_dag.fl`)

Multi-hop scatter routing is modeled as a DAG. Each route is a directed acyclic path through the exit mesh. Route nodes represent hops; hop edges carry latency and encryption overhead. Acyclicity is enforced to prevent routing loops.

```fastlang
type RouteNode = struct {
    route_id: bytes(16),
    exit_id: bytes(16),
    hop_position: u8,
    session_key_wrapped: bytes(1568),
    established_at: u64,
}

type HopEdge = struct {
    hop_index: u8,
    encryption_overhead_ns: u64,
}

dag tunnel_route {
    node RouteNode
    edge HopEdge

    enforce acyclic

    overlay hop_latency: u64 bitmask delta_curate
    overlay encryption_overhead: u64 bitmask delta_curate

    storage csr {
        hot @bram,
        warm @ddr,
        cold @nvme,
    }

    observe tunnel_route: [hop_latency, encryption_overhead] threshold: {
        anomaly_score 0.85
        baseline_window 60
    }

    traverse route_path(entry: RouteNode) -> [RouteNode]
    prove inclusion(hop: RouteNode, root: RouteNode) -> MerkleProof
}

series route_series: tunnel_route
    merkle_chain true
    lattice_imprint true
    witness_attest true
```

Key circuits: `build_route`, `teardown_route`, `reroute_hop`, `prove_route_integrity`.

---

## How Scatter-Routing Works

### Traditional VPN (Single Tunnel)
```
Device -> [All traffic] -> VPN Server -> Internet
                           ^
                           Exit node sees ALL traffic
```

### Poly VPN (Scatter-Routed)
```
Device -> Traffic split into streams via VRF
    |
    +-- Stream A (30% of traffic) -> Exit US   -> Internet
    +-- Stream B (30% of traffic) -> Exit DE   -> Internet
    +-- Stream C (30% of traffic) -> Exit SG   -> Internet
    +-- Stream D (cover traffic)  -> Exit JP   -> /dev/null
    +-- Stream E (cover traffic)  -> Exit BR   -> /dev/null
```

No single exit node sees more than ~30% of the user's traffic. Each stream is encrypted with a different ML-KEM-1024 session key per exit node. Cover traffic streams maintain constant bandwidth to prevent traffic analysis. The `ai_feed exit_selection` on the `vpn_exit_mesh` graph determines optimal stream distribution based on real-time latency, load, and jurisdiction overlays.

---

## Traffic Mimicry (Trade Secret)

Each scatter stream is shaped to match a popular application's traffic signature:

| Stream | Mimics | Characteristics |
|--------|--------|----------------|
| A | Netflix | Large packets, bursty, downstream-heavy |
| B | YouTube | Variable bitrate, HTTP/2 patterns |
| C | iCloud | Small packets, periodic sync bursts |
| D (cover) | Spotify | Constant low bitrate, occasional seeks |
| E (cover) | Zoom | Bidirectional, 30fps timing |

Matching characteristics:
- Packet sizes and distributions
- Inter-packet timing
- TLS fingerprints (JA3/JA4)
- Server Name Indication (SNI) headers
- Connection patterns (keep-alive, reconnect)

**Classification**: TRADE SECRET. Implementation details in private repos only.

---

## Key Features

### Kill Switch
- OS-level firewall rules activated when VPN connects
- All traffic blocked if VPN drops (no leak)
- Implemented at TUN/TAP level for reliability

### Split Tunneling
```yaml
split_tunnel:
  include:
    - "*.company.internal"
    - "banking.com"
  exclude:
    - "*.local"
    - "192.168.0.0/16"
    - "streaming-service.com"
```

### Multi-Hop (Built into Scatter)

Traditional VPNs add latency with multi-hop chains. Poly VPN's scatter routing is inherently multi-exit — traffic goes through multiple paths by design, modeled as the `tunnel_route` DAG. Each path through the DAG is a multi-hop route with independent PQ session keys.

### Net Shield (DNS Protection)
- DNS-over-eStream (PQ-encrypted DNS queries)
- Block lists: ads, trackers, malware domains
- Custom block/allow lists
- DNS queries scatter-routed (no single DNS provider sees all queries)

### Profiles
```yaml
profiles:
  - name: "Work"
    exits: [us-east, us-west]
    split_tunnel: work_config
    net_shield: standard

  - name: "Travel (High Security)"
    exits: [de, sg, jp, br, za]
    split_tunnel: none
    net_shield: strict
    mimicry: true
    cover_traffic: true

  - name: "Streaming"
    exits: [us-east]
    split_tunnel: streaming_config
    net_shield: standard
```

---

## FastLang Circuits

All circuits are written in FastLang `.fl` using PolyKit profiles. The build pipeline is:

```bash
estream-dev build-wasm-client --from-fl circuits/fl/ --sign key.pem --enforce-budget
```

### Client-Side Circuits (compiled to `.escd` WASM)

| Circuit | File | Purpose | Size Budget |
|---------|------|---------|-------------|
| `polyvpn_encrypt` | `polyvpn_encrypt.fl` | ML-KEM-1024 per-exit session key exchange, packet encryption | ≤128 KB |
| `polyvpn_scatter` | `polyvpn_scatter.fl` | VRF-directed stream splitting, scatter routing | ≤128 KB |
| `polyvpn_mimicry` | `polyvpn_mimicry.fl` | Traffic shaping, cover traffic generation | ≤128 KB |
| `polyvpn_killswitch` | `polyvpn_killswitch.fl` | OS firewall rule management, leak prevention | ≤64 KB |
| `polyvpn_dns` | `polyvpn_dns.fl` | DNS-over-eStream, block list enforcement | ≤128 KB |

All circuits compose PolyKit:
```fastlang
circuit polyvpn_encrypt(user_id: bytes(16), exit_pubkey: bytes(1568), packet: bytes) -> bytes
    profile poly_framework_sensitive
    composes: [polykit_identity, polykit_metering, polykit_sanitize]
    lex esn/global/org/polylabs/vpn/encrypt
    constant_time true
    observe metrics: [encrypt_ops, packet_size, latency_ns]
{
    ml_kem_encapsulate(exit_pubkey, packet)
}
```

### Server-Side Circuits (lattice-hosted)

| Circuit | File | Purpose |
|---------|------|---------|
| `polyvpn_exit_router` | `polyvpn_exit_router.fl` | Exit node traffic forwarding, session validation |
| `polyvpn_health` | `polyvpn_health.fl` | Exit node health broadcasting, attestation |
| `polyvpn_metering` | `polyvpn_metering.fl` | Per-product 8D metering (isolated) |

---

## Exit Node Network

### Node Types

| Type | Description | Use Case |
|------|-------------|----------|
| Public | Poly Labs operated | Free/Premium users |
| Partner | Third-party operated (eStream operator) | Bandwidth expansion |
| Dedicated | Customer-operated | Enterprise/Sovereign |

### Exit Node Requirements

Each exit node:
- Runs eStream node software
- Holds ML-KEM-1024 session keys (in memory only, no disk)
- Cannot see traffic content (PQ-encrypted end-to-end)
- Cannot correlate traffic across scatter streams (different session keys per stream)
- Attests to hardware integrity via PoVC
- Registers as `ExitNode` in the `vpn_exit_mesh` graph

### Geographic Distribution

Target: 40+ countries across 6 continents for comprehensive exit coverage.

---

## Platform Support

| Platform | Implementation | Status |
|----------|---------------|--------|
| macOS | Tauri desktop + Network Extension | Planned |
| Windows | Tauri desktop + WFP (Windows Filtering Platform) | Planned |
| Linux | Tauri desktop + TUN device | Planned |
| iOS | React Native + Network Extension + Rust FFI | Planned |
| Android | React Native + VpnService + Rust FFI | Planned |
| Router | OpenWrt package (enterprise) | Future |

### Core Engine (Rust)

All platforms share `poly-vpn-core` (Rust):
- PQ encryption (ML-KEM-1024 per exit)
- Scatter routing (VRF-directed stream splitting)
- Traffic mimicry profiles
- Kill switch logic
- Split tunnel rules
- DNS protection

Platform-specific code is minimal (TUN/TAP setup, OS integration).

---

## StreamSight Observability

Per-product isolated telemetry within the `polylabs.vpn.*` lex namespace.

### Telemetry Stream Paths

```
lex://estream/apps/polylabs.vpn/telemetry
lex://estream/apps/polylabs.vpn/telemetry/sli
lex://estream/apps/polylabs.vpn/metrics/baseline
lex://estream/apps/polylabs.vpn/metrics/deviations
lex://estream/apps/polylabs.vpn/incidents
lex://estream/apps/polylabs.vpn/eslm/exit_selection
lex://estream/apps/polylabs.vpn/eslm/mimicry_effectiveness
```

No telemetry path references any other Poly product. StreamSight baseline gate learns per-exit latency distributions and flags deviations.

---

## Console Widgets

| Widget ID | Category | Description |
|-----------|----------|-------------|
| `polyvpn-tunnel-latency` | observability | Scatter route latency gauge (per-exit) |
| `polyvpn-exit-health` | observability | Exit node load, capacity, and availability |
| `polyvpn-bandwidth-usage` | observability | Per-stream bandwidth distribution |
| `polyvpn-deviation-feed` | observability | StreamSight baseline deviation feed |
| `polyvpn-mimicry-score` | observability | Traffic mimicry effectiveness per stream |
| `polyvpn-dns-blocks` | observability | DNS block list hit rate |
| `polyvpn-exit-coverage` | governance | Geographic exit node distribution |
| `polyvpn-eslm-routing` | governance | AI exit selection decision audit |
| `polyvpn-sanitization-log` | governance | PII sanitization audit |

---

## Metering

| Operation | Primary Dimensions |
|-----------|-------------------|
| Tunnel traffic | Bandwidth (primary) |
| Exit node relay | Bandwidth, Operations |
| DNS queries | Operations |
| Cover traffic | Bandwidth (discounted) |
| Profile switch | Operations |
| Multi-hop overhead | Compute |

---

## Pricing

| Tier | Price | Bandwidth | Devices | Exits | Features |
|------|-------|-----------|---------|-------|----------|
| Free | $0 | 1 GB/day | 1 | 3 countries | Kill switch, basic DNS |
| Premium | $4.99/mo | Unlimited | 5 | 20+ countries | Multi-exit scatter, split tunnel, Net Shield |
| Pro | $9.99/mo | Unlimited | 10 | 40+ countries | Traffic mimicry, cover traffic, profiles |
| Enterprise | Custom | Unlimited | Unlimited | Dedicated + public | Custom exits, router support, lex bridge admin |

Tier enforcement via PolyKit `metering_graph` + `subscription_lifecycle` state machine. Each tier unlocks progressively more exits, scatter breadth, and traffic mimicry.

---

## Enterprise

Enterprise customers can opt-in to cross-product visibility via the lex bridge mechanism:

- **Lex bridge**: Bridges `esn/global/org/polylabs/vpn` to the enterprise admin namespace
- **Gating**: k-of-n admin witness attestation required to activate
- **Scope**: Org-level aggregates and RBAC policy only — individual user tunnel data is never cross-linked
- **Revocable**: Bridge can be torn down at any time by the same k-of-n witness quorum
- **Dedicated exits**: Enterprise can operate their own exit nodes registered in the `vpn_exit_mesh` graph

---

## Directory Structure

```
polyvpn/
├── circuits/fl/
│   ├── polyvpn_encrypt.fl
│   ├── polyvpn_scatter.fl
│   ├── polyvpn_mimicry.fl
│   ├── polyvpn_killswitch.fl
│   ├── polyvpn_dns.fl
│   ├── polyvpn_exit_router.fl
│   ├── polyvpn_health.fl
│   ├── polyvpn_metering.fl
│   └── graphs/
│       ├── polyvpn_exit_graph.fl
│       └── polyvpn_route_dag.fl
├── crates/
│   ├── poly-vpn-core/
│   ├── poly-exit-node/
│   └── poly-vpn-platform/
├── apps/
│   ├── desktop/          (Tauri: Mac, Win, Linux)
│   └── mobile/           (React Native + Rust FFI: iOS, Android)
├── packages/
│   ├── sdk-browser/
│   └── poly-vpn-widget/
├── docs/
│   └── ARCHITECTURE.md
├── CLAUDE.md
└── Cargo.toml
```

---

## Roadmap

### Phase 1: Core Tunnel (Q3 2026)
- `vpn_exit_mesh` graph + `tunnel_route` DAG
- FastLang circuits for encryption, scatter, kill switch
- macOS + Windows desktop client (Tauri)
- SPARK auth (`poly-vpn-v1`)
- Single-exit PQ tunnel (scaffold for scatter)
- Basic DNS protection
- StreamSight L0 metrics

### Phase 2: Scatter + Mobile (Q4 2026)
- Multi-exit scatter routing via `tunnel_route` DAG
- `ai_feed exit_selection` for optimal routing
- Split tunneling
- Profiles
- iOS + Android (React Native)
- Linux desktop

### Phase 3: Mimicry (Q1 2027)
- Traffic mimicry (trade secret activation)
- Cover traffic generation
- Net Shield (full ad/tracker/malware blocking)
- Port forwarding
- Console widgets (9 widgets)

### Phase 4: Enterprise (Q2 2027)
- Dedicated exit nodes in `vpn_exit_mesh` graph
- Enterprise admin via lex bridge (opt-in, k-of-n witness gated)
- Router firmware (OpenWrt package)
- Compliance features, SLA

---

## Stratum & Cortex Integration

All graph and DAG constructs use the full Stratum+Cortex pattern from eStream v0.9.1. This provides typed storage with AI-governed data lifecycle, privacy enforcement, and anomaly detection.

### Stratum: Typed Graph/DAG Storage

| Construct | File | Storage | Signing |
|-----------|------|---------|---------|
| `vpn_exit_mesh` | `graphs/polyvpn_exit_graph.fl` | `store graph` → CSR (BRAM/DDR/NVMe tiering) | ML-DSA-87 on all mutations |
| `tunnel_route` | `graphs/polyvpn_route_dag.fl` | `store dag` → Merkle-CSR with `enforce acyclic` | ML-DSA-87 + `attest povc { witness threshold(2,3) }` |

Both constructs maintain typed overlays with `delta_curate` for real-time state without mutating base nodes:

**Exit Mesh Overlays**: `latency_ns`, `bandwidth_mbps`, `load_pct`, `jurisdiction`, `blacklist_status`

**Route DAG Overlays**: `hop_latency`, `encryption_overhead`, `hop_status`

### Cortex: AI Governance Layer

Each `data` declaration carries a `cortex` block that governs how Cortex (the AI governance layer) handles sensitive fields:

```fastlang
data ExitNode : app v1 {
    exit_id: string, endpoint_address: string, private_key_hash: string,
    jurisdiction: string, operator_id: string, ...
}
    store graph
    govern lex esn/global/org/polylabs/vpn
    cortex {
        redact [endpoint_address, private_key_hash]
        obfuscate [jurisdiction, operator_id]
        infer on_read
        on_anomaly alert "vpn-ops"
    }
```

| Directive | Effect |
|-----------|--------|
| `redact` | Fields are stripped from Cortex inference inputs and AI feed contexts |
| `obfuscate` | Fields are hashed/tokenized before Cortex processes them |
| `infer on_read` / `on_write` | When Cortex inference triggers (read-path for exits, write-path for routes) |
| `on_anomaly alert` | Anomaly detection alerts routed to the specified ops channel |

### AI Feeds

| Feed | Graph | Purpose |
|------|-------|---------|
| `exit_selection` | `vpn_exit_mesh` | Predicts optimal multi-exit scatter routes based on latency, load, and bandwidth overlays. Confidence threshold: 0.85. |

The `exit_selection` feed provides `predict`, `suggest`, and `confidence` directives:
- **predict**: `[latency_ns, load_pct, bandwidth_mbps]` — real-time overlay forecasting
- **suggest**: `[optimal_exit_set, rebalance_trigger]` — actionable routing recommendations
- **confidence**: 0.85 — minimum confidence for AI suggestions to override heuristic fallback

### Series & Attestation

Both constructs emit append-only series with full integrity chain:

```fastlang
series exit_series: vpn_exit_mesh
    merkle_chain true
    lattice_imprint true
    witness_attest true

series route_series: tunnel_route
    merkle_chain true
    lattice_imprint true
    witness_attest true
```

- **merkle_chain**: Each mutation is Merkle-chained for tamper evidence
- **lattice_imprint**: Series state imprinted to the eStream lattice for global consistency
- **witness_attest**: Independent witnesses attest to series integrity

### Route DAG: Acyclicity & PoVC

The `tunnel_route` DAG enforces structural constraints beyond what the graph provides:

- `enforce acyclic` — prevents routing loops at the DAG level
- `sign ml_dsa_87` — every node and edge mutation is PQ-signed
- `storage merkle_csr` — Merkle-backed CSR for inclusion proofs
- `attest povc { witness threshold(2,3) }` — 2-of-3 witness quorum for route attestation

The `prove_route_integrity` circuit computes a Merkle root over all hops in a route and verifies inclusion proofs, feeding anomalies back to Cortex if verification fails.

### Privacy Flow

```
Exit registration → Cortex redacts endpoint_address, private_key_hash
                  → Cortex obfuscates jurisdiction, operator_id
                  → Overlays store real-time metrics (no PII)
                  → ai_feed sees only obfuscated/overlay data
                  → on_anomaly alerts vpn-ops (no raw fields)

Route building    → Cortex redacts hop_key on write
                  → Cortex obfuscates exit_id
                  → PoVC witnesses attest route integrity
                  → Series chain provides tamper evidence
```

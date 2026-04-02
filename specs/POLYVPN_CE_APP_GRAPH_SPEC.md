# PolyVPN CE App Graph Specification

| Field | Value |
|-------|-------|
| **Version** | v0.1.0 |
| **Status** | Draft |
| **Lex Namespace** | `polylabs/polyvpn` |
| **App Graph** | `circuits/fl/polyvpn_app_graph.fl` |
| **CE Meaning** | `circuits/fl/polyvpn_meaning.fl` |
| **Upstream Dependency** | eStream v0.22.0+, PolyKit v0.1.0+ |

---

## 1. Module Inventory

PolyVPN comprises 11 modules (9 circuits + 2 graphs):

| Module | Type | Description |
|--------|------|-------------|
| `polyvpn_dns` | Circuit | PQ-encrypted DNS resolution — DoH/DoT with ML-KEM-1024, DNS leak prevention, censorship-resistant resolvers |
| `polyvpn_encrypt` | Circuit | Tunnel encryption — ML-KEM-1024 key exchange, AES-256-GCM/ChaCha20-Poly1305, per-packet nonce, forward secrecy rotation |
| `polyvpn_incognito` | Circuit | Session unlinkability — ephemeral SPARK identities per tunnel, no cross-session correlation, IP leak prevention |
| `polyvpn_killswitch` | Circuit | Kill switch enforcement — OS-level firewall rules, pre-connect blocking, IPv6 leak prevention, crash recovery auto-reconnect |
| `polyvpn_metering` | Circuit | Usage metering — per-tunnel bandwidth, connection duration, relay hop count, tier enforcement |
| `polyvpn_mimicry` | Circuit | Traffic mimicry engine — protocol camouflage (HTTPS, WebSocket, QUIC shapes), timing jitter injection, DPI evasion fingerprinting |
| `polyvpn_platform_health` | Circuit | Platform health — tunnel stability metrics, relay node availability, latency percentile tracking, certificate rotation scheduling |
| `polyvpn_rbac` | Circuit | Role-based access — personal user, family plan member, enterprise admin, network-only profiles |
| `polyvpn_scatter` | Circuit | Scatter-routed tunneling — multi-hop relay selection, geographic diversity enforcement, k-of-n path redundancy |
| `exit_graph` | Graph | Exit node topology graph — exit node locations, capacity, load, trust scores, jurisdiction metadata |
| `route_dag` | Graph | Route selection DAG — multi-hop paths, latency weights, censorship bypass routes, failover edges |

---

## 2. CE Meaning Domains

### 2.1 `network/tunnel_health`

Monitors tunnel stability and performance derived from latency measurements, packet loss detection, and relay health probes.

| Signal | Source | Meaning |
|--------|--------|---------|
| Latency spike detected | `polyvpn_platform_health` | Tunnel degradation — route_dag failover candidate |
| Packet loss exceeds threshold | `polyvpn_platform_health` | Relay node congestion or ISP throttling detected |
| Tunnel reconnection event | `polyvpn_killswitch` | Connection drop — kill switch activated, auto-recovery in progress |
| Exit node capacity breach | `exit_graph` | Exit node overloaded — rebalance to alternate exit |
| Forward secrecy rotation failure | `polyvpn_encrypt` | Key rotation timeout — potential MITM or relay compromise |

### 2.2 `network/censorship_detection`

Tracks censorship events, block patterns, and DPI evasion effectiveness across jurisdictions.

| Signal | Source | Meaning |
|--------|--------|---------|
| DPI fingerprint match | `polyvpn_mimicry` | Deep packet inspection attempt detected — protocol switch required |
| DNS poisoning detected | `polyvpn_dns` | Resolver returning spoofed records — failover to encrypted resolver |
| TCP RST injection | `polyvpn_scatter` | Active connection termination by middlebox — reroute via alternate path |
| Protocol block pattern | `polyvpn_mimicry` | Specific protocol shape blocked in jurisdiction — update mimicry profile |
| Geofence restriction event | `exit_graph` | Exit node jurisdiction blocked — geographic rerouting triggered |

### 2.3 `network/mimicry_effectiveness`

Observes protocol camouflage success rates and traffic analysis resistance across mimicry profiles.

| Signal | Source | Meaning |
|--------|--------|---------|
| Camouflage success rate | `polyvpn_mimicry` | Percentage of sessions passing DPI without detection |
| Timing jitter entropy score | `polyvpn_mimicry` | Statistical distinguishability of injected timing from genuine traffic |
| Protocol shape mismatch | `polyvpn_mimicry` | Mimicry profile diverging from real protocol baseline — recalibration needed |
| Bandwidth fingerprint anomaly | `polyvpn_scatter` | Traffic volume pattern reveals VPN usage — padding strategy adjustment |
| Cover traffic ratio | `polyvpn_scatter` | Ratio of cover to real traffic — efficiency vs undetectability tradeoff |

---

## 3. Noise Filter

Suppress high-frequency, low-signal events to prevent CE observation saturation:

| Suppressed Event | Reason |
|------------------|--------|
| Keepalive packets (tunnel heartbeat) | Periodic liveness probe — no user action or security meaning |
| DNS prefetch / speculative resolution | Background optimization — not user-initiated |
| Metering counter increment | Raw counter — aggregate in metering circuit, not CE |
| Relay health ping ACK | Scatter topology protocol noise |

Signal through (always observe):

| Signal Event | Reason |
|--------------|--------|
| Tunnel failure / kill switch activation | Immediate connectivity and security meaning |
| Censorship event (DPI match, DNS poison, TCP RST) | Active adversary indicator |
| Mimicry profile failure / detection | Camouflage integrity breach |
| Exit node capacity breach | Service quality degradation |
| Forward secrecy rotation failure | Cryptographic security boundary event |
| Protocol shape block pattern | Jurisdiction-level censorship escalation |

---

## 4. SME Panels

### 4.1 Network Security Posture Panel

Convenes on tunnel health threshold crossings: latency > 200ms sustained, packet loss > 2%, forward secrecy rotation failure, or exit node trust score degradation.

| Panelist | Focus |
|----------|-------|
| **Security Advocate** | Cryptographic integrity, MITM risk assessment, relay trust evaluation |
| **Performance Advocate** | Latency impact on user experience, route optimization tradeoffs |
| **Synthesis** | Risk-adjusted route recommendation: reroute now vs degrade gracefully vs alert user |

### 4.2 Censorship Evasion Panel

Convenes on censorship events: DPI fingerprint match, DNS poisoning cluster, TCP RST injection spike, or protocol block pattern across multiple users in a jurisdiction.

| Panelist | Focus |
|----------|-------|
| **Evasion Advocate** | Mimicry profile rotation urgency, protocol shape diversity, timing entropy |
| **Stability Advocate** | Disruption cost of profile switch, user session continuity, bandwidth overhead |
| **Synthesis** | Classify event as targeted probe vs broad censorship escalation; recommend mimicry update schedule |

---

## 5. Bridge Edges

### 5.1 PolyKit Incognito Bridge

| Direction | Shared Fields | Purpose |
|-----------|---------------|---------|
| `polyvpn` → `polykit/incognito` | `ephemeral_spark_id`, `session_unlinkability_proof`, `identity_rotation_schedule` | Session isolation — PolyKit incognito provides SPARK ephemeral identities consumed by VPN tunnel establishment |
| `polykit/incognito` → `polyvpn` | `tunnel_session_token`, `exit_jurisdiction` | Tunnel metadata — PolyVPN informs incognito layer of active tunnel context for cross-product unlinkability enforcement |

### 5.2 eStream Scatter Routing Bridge

| Direction | Shared Fields | Purpose |
|-----------|---------------|---------|
| `polyvpn` → `estream/scatter` | `route_dag_snapshot`, `relay_health_vector`, `hop_count` | Route coordination — PolyVPN shares relay topology with eStream scatter for platform-wide routing optimization |
| `estream/scatter` → `polyvpn` | `scatter_node_capacity`, `geographic_diversity_map`, `trust_attestation` | Node discovery — eStream scatter provides relay node metadata for VPN path selection |

---

## 6. Strategic Grants

| Grantor | Grant | Purpose |
|---------|-------|---------|
| **eStream** | `scatter-cas`, `ml-kem-1024`, `ml-dsa-87`, `spark`, `delta-curate`, `rbac`, `alert-pipeline`, `ssm`, `scatter-routing` | Platform primitives for PQ encryption, identity, audit, access control, CE, multi-hop routing |
| **Paragon** | `jurisdiction_compliance_rules`, `geographic_policy_engine` | Jurisdiction-aware routing policies, compliance-driven exit node selection for family office clients |

---

## 7. Platform Graph Registration

### Circuit Counts

| Category | Count |
|----------|-------|
| App Graph modules | 11 (9 circuits + 2 graphs) |
| CE meaning domains | 3 |
| SME panels | 2 |
| Bridge edges | 2 (PolyKit incognito, eStream scatter) |
| **Total** | **11 modules** |

### Capability Inventory Update

```
polyvpn: {
    modules: 11,
    circuits: 9,
    graphs: 2,
    ce_meaning_domains: 3,
    sme_panels: 2,
    bridge_edges: 2,
    mimicry_profiles: ["https", "websocket", "quic"],
    noise_filter_suppressed: 4,
    noise_filter_signaled: 6,
}
```

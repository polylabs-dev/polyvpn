# Q VPN

**GitHub**: [polylabs-dev/qvpn](https://github.com/polylabs-dev/qvpn)
**Platform**: eStream v0.22.0
**Depends on**: QKit, eStream graph/DAG constructs

100% FastLang. No hand-written Rust.

## Purpose

Post-quantum encrypted, scatter-routed VPN with traffic mimicry. Traffic is split across multiple exit nodes simultaneously — no single exit sees the complete picture. All crypto compiled from FastLang via FLIR codegen.

## Zero-Linkage Privacy

HKDF context: `q-vpn-v1`. User identities are completely isolated from all other Q products. StreamSight telemetry stays within `polyqlabs.vpn.*` lex namespace. Metering under `polyqlabs.vpn.metering`. Billing via blinded tokens.

## Structure

- `circuits/fl/` — FastLang circuit definitions (encrypt, scatter, mimicry, kill switch, DNS, metering, incognito, RBAC)
- `circuits/fl/graphs/` — Graph/DAG constructs (vpn_exit_mesh, tunnel_route)
- `estream.toml` — Platform v0.22.0 component manifest
- `apps/desktop/` — Tauri desktop (Mac, Win, Linux)
- `apps/mobile/` — React Native + Rust FFI (iOS, Android)
- `docs/` — Architecture and design documents

> **Note**: `crates/` and `packages/` are legacy scaffolding superseded by FLIR codegen. All logic lives in FastLang circuits.

## Key Graphs

- `graph vpn_exit_mesh` — exit node topology with `ai_feed exit_selection`, overlays: latency_ns, bandwidth_mbps, load_pct, jurisdiction, blacklist_status
- `dag tunnel_route` — multi-hop scatter routing DAG, `enforce acyclic`, overlays: hop_latency, encryption_overhead

## Commit Convention

Commit to the GitHub issue or epic the work was done under.

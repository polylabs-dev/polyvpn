# Poly VPN

**GitHub**: [polylabs-dev/polyvpn](https://github.com/polylabs-dev/polyvpn)
**Platform**: eStream v0.8.3
**Depends on**: PolyKit v0.3.0, eStream graph/DAG constructs

## Purpose

Post-quantum encrypted, scatter-routed VPN with traffic mimicry. Traffic is split across multiple exit nodes simultaneously — no single exit sees the complete picture. All crypto in Rust/WASM; TypeScript is DOM-only.

## Zero-Linkage Privacy

HKDF context: `poly-vpn-v1`. User identities are completely isolated from all other Poly products. StreamSight telemetry stays within `polylabs.vpn.*` lex namespace. Metering under `polylabs.vpn.metering`. Billing via blinded tokens.

## Structure

- `circuits/fl/` — FastLang circuit definitions (encrypt, scatter, mimicry, kill switch, DNS, metering)
- `circuits/fl/graphs/` — Graph/DAG constructs (vpn_exit_mesh, tunnel_route)
- `crates/` — Rust crates (poly-vpn-core, poly-exit-node, poly-vpn-platform)
- `apps/desktop/` — Tauri desktop (Mac, Win, Linux)
- `apps/mobile/` — React Native + Rust FFI (iOS, Android)
- `packages/` — TypeScript SDKs and console widgets
- `docs/` — Architecture and design documents

## Key Graphs

- `graph vpn_exit_mesh` — exit node topology with `ai_feed exit_selection`, overlays: latency_ns, bandwidth_mbps, load_pct, jurisdiction, blacklist_status
- `dag tunnel_route` — multi-hop scatter routing DAG, `enforce acyclic`, overlays: hop_latency, encryption_overhead

## Commit Convention

Commit to the GitHub issue or epic the work was done under.

## Cross-Repo Coordination

This repo is part of the [polylabs-dev](https://github.com/polylabs-dev) organization, coordinated through the **AI Toolkit hub** at `toddrooke/ai-toolkit/`.

For cross-repo context, strategic priorities, and the master work queue:
- `toddrooke/ai-toolkit/CLAUDE-CONTEXT.md` — org map and priorities
- `toddrooke/ai-toolkit/scratch/BACKLOG.md` — master backlog
- `toddrooke/ai-toolkit/repos/polylabs-dev.md` — this org's status summary

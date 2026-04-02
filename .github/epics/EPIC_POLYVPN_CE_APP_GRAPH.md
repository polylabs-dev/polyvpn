# PolyVPN CE + App Graph Integration

> **Status**: Proposed
> **Priority**: P1
> **Spec**: `specs/POLYVPN_CE_APP_GRAPH_SPEC.md`
> **Estimated Effort**: 3-4 weeks

---

## Overview

Implement the PolyVPN CE App Graph: 11 modules (9 circuits + 2 graphs), 3 CE meaning domains, 2 SME panels, and 2 bridge edges (PolyKit incognito, eStream scatter routing). Delivers cognitive observability for tunnel health, censorship detection, and mimicry effectiveness.

## Task Checklist

- [ ] **App Graph scaffold** — Create `polyvpn_app_graph.fl` with `make_polyvpn_module` helper and 11 `ModuleNode` definitions
- [ ] **Intra-graph REQUIRES edges** — Wire 10 dependency edges (scatter→route_dag, scatter→exit_graph, scatter→encrypt, mimicry→encrypt, dns→encrypt, incognito→encrypt, killswitch→scatter, metering→scatter, platform_health→exit_graph, platform_health→route_dag)
- [ ] **CE meaning domains** — Implement 3 domains in `polyvpn_meaning.fl`: `network/tunnel_health`, `network/censorship_detection`, `network/mimicry_effectiveness`
- [ ] **Noise filter** — 4 suppression rules (keepalive, DNS prefetch, metering counter, relay ping ACK) + 6 signal rules
- [ ] **SME panels** — Network Security Posture panel + Censorship Evasion panel with FOR/AGAINST deliberation
- [ ] **Bridge edges** — PolyKit incognito bridge (ephemeral SPARK session isolation) + eStream scatter routing bridge (relay topology coordination)
- [ ] **Golden tests** — Module registration (11 nodes, 10 edges), bridge edge connectivity, meaning domain validation, noise filter counts, SME panel config
- [ ] **Platform Graph update** — Register polyvpn in `estream/circuits/services/capabilities/` inventory with module counts
- [ ] **Spec review** — Finalize `POLYVPN_CE_APP_GRAPH_SPEC.md` from Draft → Approved

## Acceptance Criteria

- `polyvpn_app_graph_register` adds 11 nodes and 10 edges to CsrStorage
- `polyvpn_register_bridge_edges` connects to PolyKit incognito + eStream scatter
- `polyvpn_register_ce` registers 3 domains, 1 noise filter, 2 SME panels
- All golden tests pass
- Zero-linkage isolation maintained (no cross-product lex leakage)

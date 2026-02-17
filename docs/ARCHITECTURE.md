# Poly VPN Architecture

**Version**: 1.0
**Last Updated**: February 2026
**Platform**: eStream v0.8.1

---

## Overview

Poly VPN provides device-level post-quantum encrypted privacy for all network traffic. Unlike traditional VPNs that route traffic through a single tunnel to a single exit node, Poly VPN scatter-routes traffic across multiple exit nodes simultaneously. No single exit node sees the complete traffic picture.

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        User Device                           │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐   │
│  │                 Poly VPN Client                       │   │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐          │   │
│  │  │ UI       │  │ Profiles │  │ Net      │          │   │
│  │  │ (Tauri)  │  │ Manager  │  │ Shield   │          │   │
│  │  └────┬─────┘  └────┬─────┘  └────┬─────┘          │   │
│  │       │              │              │                │   │
│  │  ┌────┴──────────────┴──────────────┴─────────────┐ │   │
│  │  │           poly-vpn-core (Rust)                  │ │   │
│  │  │  SPARK Auth | PQ Encrypt | Split Tunnel | Kill  │ │   │
│  │  └──────────────────────┬─────────────────────────┘ │   │
│  └─────────────────────────┼───────────────────────────┘   │
│                             │                               │
│  ┌─────────────────────────┴───────────────────────────┐   │
│  │              TUN/TAP Interface                       │   │
│  │         (captures all device traffic)                │   │
│  └─────────────────────────┬───────────────────────────┘   │
└─────────────────────────────┼───────────────────────────────┘
                              │
                     PQ Encrypt + Scatter
                              │
              ┌───────────────┼───────────────┐
              │               │               │
        ┌─────┴─────┐  ┌─────┴─────┐  ┌─────┴─────┐
        │ Exit Node │  │ Exit Node │  │ Exit Node │
        │ A (US)    │  │ B (DE)    │  │ C (SG)    │
        │ Netflix   │  │ YouTube   │  │ iCloud    │
        │ mimicry   │  │ mimicry   │  │ mimicry   │
        └─────┬─────┘  └─────┬─────┘  └─────┬─────┘
              │               │               │
              v               v               v
           Internet        Internet        Internet
```

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
Device -> Traffic split into streams
    |
    +-- Stream A (30% of traffic) -> Exit US   -> Internet
    +-- Stream B (30% of traffic) -> Exit DE   -> Internet
    +-- Stream C (30% of traffic) -> Exit SG   -> Internet
    +-- Stream D (cover traffic)  -> Exit JP   -> /dev/null
    +-- Stream E (cover traffic)  -> Exit BR   -> /dev/null
```

No single exit node sees more than ~30% of the user's traffic. The streams are encrypted with different ML-KEM-1024 session keys per exit node. Cover traffic streams maintain constant bandwidth to prevent traffic analysis.

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
# Profile configuration
split_tunnel:
  include:
    - "*.company.internal"  # Route through VPN
    - "banking.com"
  exclude:
    - "*.local"             # Direct connection
    - "192.168.0.0/16"
    - "streaming-service.com"  # Direct for performance
```

### Multi-Hop (Built into Scatter)
Traditional VPNs add latency with multi-hop chains. Poly VPN's scatter routing is inherently multi-exit -- traffic goes through multiple paths by design, not as an add-on feature.

### Net Shield (DNS Protection)
- DNS-over-eStream (PQ-encrypted DNS queries)
- Block lists: ads, trackers, malware domains
- Custom block/allow lists
- DNS queries scatter-routed (no single DNS provider sees all queries)

### Profiles
```yaml
# Saved connection profiles
profiles:
  - name: "Work"
    exits: [us-east, us-west]
    split_tunnel: work_config
    net_shield: standard
    
  - name: "Travel (High Security)"
    exits: [de, sg, jp, br, za]  # 5 exits
    split_tunnel: none  # All traffic through VPN
    net_shield: strict
    mimicry: true
    cover_traffic: true
    
  - name: "Streaming"
    exits: [us-east]  # Single exit for geo-unlock
    split_tunnel: streaming_config
    net_shield: standard
```

---

## Platform Support

| Platform | Implementation | Status |
|----------|---------------|--------|
| macOS | Tauri desktop + Network Extension | Planned |
| Windows | Tauri desktop + WFP (Windows Filtering Platform) | Planned |
| Linux | Tauri desktop + TUN device | Planned |
| iOS | React Native + Network Extension | Planned |
| Android | React Native + VpnService | Planned |
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

## ESCIR Circuits

### VPN Router Circuit

```yaml
escir: "0.8.1"
name: poly-vpn-router
version: "1.0.0"
lex: polylabs.vpn

stream:
  - topic: "polylabs.vpn.{device_id}.tunnel"
    pattern: scatter
    retention: none  # Transit only
    signature_required: true

  - topic: "polylabs.vpn.{device_id}.dns"
    pattern: request_reply
    retention: none
    signature_required: true

  - topic: "polylabs.vpn.exits.status"
    pattern: fanout
    retention: 5m
    # Exit node health/load broadcasting

  - topic: "polylabs.vpn.{device_id}.metering"
    pattern: scatter
    retention: 30d
    hash_chain: true
```

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
- Cannot correlate traffic across streams (different sessions)
- Attests to hardware integrity via PoVC

### Geographic Distribution

Target: 40+ countries across 6 continents for comprehensive exit coverage.

---

## Metering

| Operation | Primary Dimensions |
|-----------|-------------------|
| Tunnel traffic | Bandwidth (primary) |
| Exit node | Bandwidth, Operations |
| DNS queries | Operations |
| Cover traffic | Bandwidth (discounted) |
| Profile switch | Operations |

---

## Pricing

| Tier | Bandwidth | Devices | Exits | Features |
|------|-----------|---------|-------|----------|
| Free | 1GB/day | 1 | 3 countries | Kill switch, basic DNS |
| Premium ($4.99) | Unlimited | 5 | 20+ countries | Multi-exit, split tunnel, Net Shield |
| Pro ($9.99) | Unlimited | 10 | 40+ countries | Mimicry, cover traffic, profiles |
| Enterprise | Unlimited | Unlimited | Dedicated + public | Custom, router support |

---

## Roadmap

### Phase 1: Core (Q3 2026)
- macOS + Windows desktop client
- Single-exit PQ tunnel
- Kill switch
- Basic DNS protection
- SPARK auth

### Phase 2: Scatter (Q4 2026)
- Multi-exit scatter routing
- Split tunneling
- Profiles
- iOS + Android
- Linux desktop

### Phase 3: Advanced (Q1 2027)
- Traffic mimicry (trade secret activation)
- Cover traffic
- Net Shield (full ad/tracker blocking)
- Port forwarding

### Phase 4: Enterprise (2027)
- Dedicated exit nodes
- Router firmware
- Enterprise admin console
- Compliance features
- SLA

---

## Related Documents

- [polylabs/business/PRODUCT_FAMILY.md] -- Product specifications
- [polylabs/business/PROTON_REFERENCE.md] -- Proton VPN reference
- [polylabs/business/STRATEGY.md] -- Overall strategy

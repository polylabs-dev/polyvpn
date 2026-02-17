# Poly VPN

Post-quantum encrypted, scatter-routed VPN built on eStream v0.8.1.

## Overview

Poly VPN provides device-level PQ-encrypted privacy for all traffic -- not a single tunnel but scatter-routed across multiple exit nodes simultaneously. Traffic mimicry (trade secret) makes VPN traffic look like normal app usage.

## Architecture

```
Device Kernel/Tun Interface
    |
    +-- SPARK Auth (ML-DSA-87 biometric)
    |
    +-- PQ Encrypt (ML-KEM-1024 per exit node)
    |
    v
Scatter Router (VRF-directed multi-path)
    |
    +-- Exit Node A (US) -- looks like Netflix
    +-- Exit Node B (EU) -- looks like YouTube
    +-- Exit Node C (APAC) -- looks like iCloud
    |
    v
Internet (traffic appears from multiple unrelated sources)
```

## Key Differentiators

| Feature | Traditional VPN | Poly VPN |
|---------|----------------|----------|
| Encryption | Classical (WireGuard/IKEv2) | **PQ** (ML-KEM-1024) |
| Tunnel | Single to exit node | **Multi-exit scatter** |
| Traffic analysis | VPN protocol detectable | **Traffic mimicry** |
| Exit visibility | Single exit sees all traffic | **No exit sees complete picture** |
| Kill switch | Yes | Yes |
| Split tunneling | Sometimes | Yes |

## Platforms

- macOS, Windows, Linux (Tauri desktop)
- iOS, Android (React Native + Rust FFI)
- Router firmware (enterprise)

## No REST API

All management uses the eStream Wire Protocol. No REST/HTTP endpoints.

## Platform

- eStream v0.8.1
- ESCIR SmartCircuits for routing/metering
- ML-KEM-1024, ML-DSA-87, SHA3-256
- 8-Dimension metering
- L2 multi-token payments

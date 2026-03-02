use estream_test::{
    Journey, JourneyParty, JourneyStep, StepAction, JourneyMetrics,
    assert_metric_emitted, assert_blinded, assert_povc_witness,
};
use estream_test::convoy::{ConvoyContext, ConvoyResult};
use estream_test::stratum::{StratumVerifier, CsrTier, SeriesMerkleChain};
use estream_test::cortex::{CortexVisibility, RedactPolicy, ObfuscatePolicy};
use estream_test::scatter_route::{TunnelConfig, MimicryProfile, KillSwitch};

pub struct PolyvpnJourney;

impl Journey for PolyvpnJourney {
    fn name(&self) -> &str {
        "polyvpn_e2e"
    }

    fn description(&self) -> &str {
        "End-to-end journey for Polyvpn: tunnel connect, traffic mimicry, DNS resolution, scatter-routing, killswitch, jurisdiction diversity"
    }

    fn parties(&self) -> Vec<JourneyParty> {
        vec![
            JourneyParty::new("alice")
                .with_spark_context("poly-vpn-v1")
                .with_role("client"),
            JourneyParty::new("bob")
                .with_spark_context("poly-vpn-v1")
                .with_role("exit_node"),
            JourneyParty::new("charlie")
                .with_spark_context("poly-vpn-v1")
                .with_role("relay_node"),
        ]
    }

    fn steps(&self) -> Vec<JourneyStep> {
        vec![
            // Step 1: Alice connects a PQ-encrypted tunnel
            JourneyStep::new("alice_connects_tunnel")
                .party("alice")
                .action(StepAction::Execute(|ctx: &mut ConvoyContext| {
                    let tunnel = ctx.polyvpn().connect(TunnelConfig {
                        kem_algo: "ml-kem-1024",
                        min_hops: 3,
                        jurisdiction_exclude: vec!["FVEY"],
                        mimicry_enabled: true,
                    })?;

                    ctx.set("tunnel_id", &tunnel.id);
                    ctx.set("entry_node", &tunnel.entry_node_id);

                    assert!(tunnel.established);
                    assert_eq!(tunnel.kem_algo, "ml-kem-1024");
                    assert!(tunnel.hop_count >= 3);
                    assert!(tunnel.pq_handshake_complete);

                    assert_metric_emitted!(ctx, "polyvpn.tunnel.connected", {
                        "kem_algo" => "ml-kem-1024",
                        "hop_count" => &tunnel.hop_count.to_string(),
                    });

                    assert_povc_witness!(ctx, "polyvpn.tunnel.connect", {
                        witness_type: "tunnel_establishment",
                        tunnel_id: &tunnel.id,
                    });

                    Ok(())
                }))
                .timeout_ms(12_000),

            // Step 2: Traffic mimicry activation and verification
            JourneyStep::new("traffic_mimicry_activated")
                .party("alice")
                .depends_on(&["alice_connects_tunnel"])
                .action(StepAction::Execute(|ctx: &mut ConvoyContext| {
                    let tunnel_id = ctx.get::<String>("tunnel_id");

                    let mimicry = ctx.polyvpn().activate_mimicry(
                        &tunnel_id,
                        MimicryProfile::Https,
                    )?;

                    assert!(mimicry.active);
                    assert_eq!(mimicry.profile, MimicryProfile::Https);
                    assert!(mimicry.tls_fingerprint_matches);

                    let analysis = ctx.polyvpn().analyze_traffic_pattern(&tunnel_id)?;
                    assert!(
                        !analysis.vpn_detectable,
                        "Traffic must be indistinguishable from HTTPS"
                    );
                    assert!(analysis.packet_size_variance_low);
                    assert!(analysis.timing_mimics_browser);

                    assert_metric_emitted!(ctx, "polyvpn.mimicry.activated", {
                        "profile" => "https",
                        "detectable" => "false",
                    });

                    assert_blinded!(ctx, "polyvpn.mimicry.activated", {
                        field: "client_ip",
                        blinding: "absent",
                    });

                    Ok(())
                }))
                .timeout_ms(10_000),

            // Step 3: DNS resolution through PQ-encrypted scatter path
            JourneyStep::new("dns_resolved")
                .party("alice")
                .depends_on(&["traffic_mimicry_activated"])
                .action(StepAction::Execute(|ctx: &mut ConvoyContext| {
                    let tunnel_id = ctx.get::<String>("tunnel_id");

                    let dns_result = ctx.polyvpn().resolve_dns(
                        &tunnel_id,
                        "example.com",
                    )?;

                    assert!(dns_result.resolved);
                    assert!(dns_result.encrypted_query);
                    assert!(dns_result.no_plaintext_leak);
                    assert!(dns_result.scatter_routed);
                    assert!(!dns_result.ip_addresses.is_empty());

                    // DNS must not leak to ISP
                    assert!(dns_result.isp_visible_queries == 0);

                    assert_metric_emitted!(ctx, "polyvpn.dns.resolved", {
                        "encrypted" => "true",
                        "scatter_routed" => "true",
                    });

                    assert_blinded!(ctx, "polyvpn.dns.resolved", {
                        field: "queried_domain",
                        blinding: "hmac_sha3",
                    });

                    assert_povc_witness!(ctx, "polyvpn.dns", {
                        witness_type: "dns_resolution",
                        tunnel_id: &tunnel_id,
                    });

                    Ok(())
                }))
                .timeout_ms(8_000),

            // Step 4: Scatter-routed data transfer through relay nodes
            JourneyStep::new("scatter_routed")
                .party("alice")
                .depends_on(&["dns_resolved"])
                .action(StepAction::Execute(|ctx: &mut ConvoyContext| {
                    let tunnel_id = ctx.get::<String>("tunnel_id");

                    let payload = ctx.generate_test_payload(1024 * 256); // 256 KiB
                    let transfer = ctx.polyvpn().send_data(
                        &tunnel_id,
                        &payload,
                    )?;

                    assert!(transfer.scatter_routed);
                    assert!(transfer.relay_count >= 2);
                    assert!(transfer.bytes_transferred == payload.len());

                    // No single relay sees the full payload
                    for relay in &transfer.relays {
                        assert!(relay.partial_view_only);
                        assert!(relay.encrypted_segment);
                    }

                    // Verify Charlie acted as a relay
                    let charlie_id = ctx.party_id("charlie");
                    let charlie_relay = transfer.relays.iter()
                        .find(|r| r.node_id == charlie_id);
                    assert!(charlie_relay.is_some(), "Charlie should be in the relay path");

                    assert_metric_emitted!(ctx, "polyvpn.data.scatter_routed", {
                        "relay_count" => &transfer.relay_count.to_string(),
                    });

                    assert_blinded!(ctx, "polyvpn.data.scatter_routed", {
                        field: "payload_content",
                        blinding: "absent",
                    });

                    Ok(())
                }))
                .timeout_ms(15_000),

            // Step 5: Killswitch test — verify no leak on tunnel drop
            JourneyStep::new("killswitch_tested")
                .party("alice")
                .depends_on(&["scatter_routed"])
                .action(StepAction::Execute(|ctx: &mut ConvoyContext| {
                    let tunnel_id = ctx.get::<String>("tunnel_id");

                    let ks_result = ctx.polyvpn().test_killswitch(&tunnel_id)?;

                    assert!(ks_result.killswitch_armed);

                    // Simulate tunnel drop
                    ctx.polyvpn().simulate_tunnel_drop(&tunnel_id)?;

                    assert!(ks_result.all_traffic_blocked);
                    assert!(ks_result.dns_leak_detected == false);
                    assert!(ks_result.ipv6_leak_detected == false);
                    assert!(ks_result.webrtc_leak_detected == false);

                    // Re-establish and verify reconnect
                    let reconnect = ctx.polyvpn().reconnect(&tunnel_id)?;
                    assert!(reconnect.established);
                    assert!(reconnect.session_resumed);

                    assert_metric_emitted!(ctx, "polyvpn.killswitch.tested", {
                        "leaks_detected" => "0",
                        "reconnected" => "true",
                    });

                    assert_povc_witness!(ctx, "polyvpn.killswitch", {
                        witness_type: "leak_test",
                        tunnel_id: &tunnel_id,
                    });

                    Ok(())
                }))
                .timeout_ms(15_000),

            // Step 6: Verify jurisdiction diversity and Stratum storage
            JourneyStep::new("verify_jurisdiction_diversity")
                .party("alice")
                .depends_on(&["killswitch_tested"])
                .action(StepAction::Execute(|ctx: &mut ConvoyContext| {
                    let tunnel_id = ctx.get::<String>("tunnel_id");

                    let topology = ctx.polyvpn().tunnel_topology(&tunnel_id)?;

                    let jurisdictions: Vec<&str> = topology.nodes.iter()
                        .map(|n| n.jurisdiction.as_str())
                        .collect();

                    // No two consecutive hops in the same jurisdiction
                    for window in jurisdictions.windows(2) {
                        assert_ne!(
                            window[0], window[1],
                            "Consecutive hops must be in different jurisdictions"
                        );
                    }

                    // FVEY exclusion enforced
                    let fvey = ["US", "GB", "CA", "AU", "NZ"];
                    for j in &jurisdictions {
                        assert!(
                            !fvey.contains(j),
                            "FVEY jurisdiction {} found in tunnel path",
                            j
                        );
                    }

                    // Stratum verification for session metadata
                    let stratum = StratumVerifier::new(ctx);
                    let csr = stratum.verify_csr_tiers(&tunnel_id)?;
                    assert!(csr.tier_matches(CsrTier::Ephemeral));
                    assert!(csr.shard_distribution_valid);

                    let merkle = stratum.verify_series_merkle_chain(&tunnel_id)?;
                    assert!(merkle.chain_intact);
                    assert!(merkle.root_hash_valid);

                    assert_metric_emitted!(ctx, "polyvpn.jurisdiction.verified", {
                        "fvey_excluded" => "true",
                        "diversity_ok" => "true",
                    });

                    Ok(())
                }))
                .timeout_ms(10_000),

            // Step 7: Verify blind telemetry and Cortex visibility
            JourneyStep::new("verify_blind_telemetry")
                .party("alice")
                .depends_on(&["verify_jurisdiction_diversity"])
                .action(StepAction::Execute(|ctx: &mut ConvoyContext| {
                    let telemetry = ctx.streamsight().drain_telemetry("poly-vpn-v1");

                    for event in &telemetry {
                        assert_blinded!(ctx, &event.event_type, {
                            field: "user_id",
                            blinding: "hmac_sha3",
                        });

                        assert_blinded!(ctx, &event.event_type, {
                            field: "client_ip",
                            blinding: "absent",
                        });

                        assert_blinded!(ctx, &event.event_type, {
                            field: "destination_url",
                            blinding: "absent",
                        });

                        assert_blinded!(ctx, &event.event_type, {
                            field: "dns_queries",
                            blinding: "absent",
                        });
                    }

                    let cortex = CortexVisibility::new(ctx);
                    cortex.assert_redacted("polyvpn", RedactPolicy::ContentFields)?;
                    cortex.assert_obfuscated("polyvpn", ObfuscatePolicy::PartyIdentifiers)?;

                    assert!(telemetry.len() >= 6, "Expected at least 6 telemetry events");

                    for event in &telemetry {
                        assert!(
                            event.namespace.starts_with("poly-vpn-v1"),
                            "Telemetry leaked outside poly-vpn-v1 namespace: {}",
                            event.namespace
                        );
                    }

                    Ok(())
                }))
                .timeout_ms(5_000),
        ]
    }

    fn metrics(&self) -> JourneyMetrics {
        JourneyMetrics {
            expected_events: vec![
                "polyvpn.tunnel.connected",
                "polyvpn.mimicry.activated",
                "polyvpn.dns.resolved",
                "polyvpn.data.scatter_routed",
                "polyvpn.killswitch.tested",
                "polyvpn.jurisdiction.verified",
            ],
            max_duration_ms: 90_000,
            required_povc_witnesses: 4,
            lex_namespace: "poly-vpn-v1",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use estream_test::convoy::ConvoyRunner;

    #[tokio::test]
    async fn run_polyvpn_journey() {
        let runner = ConvoyRunner::new()
            .with_scatter_route()
            .with_streamsight("poly-vpn-v1")
            .with_stratum()
            .with_cortex()
            .with_killswitch();

        runner.run(PolyvpnJourney).await.expect("Polyvpn journey failed");
    }
}

//! Differential test: `vnkey_engine::SmartSwitchTable` vs the vendored C++
//! `getAppInputMethodStatus`/`setAppInputMethodStatus`, driven with identical
//! get/set operation sequences, plus a byte-exact blob round-trip through the
//! oracle.
//!
//! The oracle's smart-switch state (map + 1-entry read cache) is global; each
//! `Oracle::lock` runs `ok_reset`, which clears the map and (via the
//! VNKEY-ORACLE-PATCH) the read cache, so both engines start every case from
//! the same empty state.

use openkey_oracle::{OkOptions, Oracle};
use proptest::prelude::*;
use vnkey_engine::SmartSwitchTable;

/// A small bundle-id pool so repeats/cache-hits are common.
const BUNDLES: [&str; 5] = [
    "com.apple.Safari",
    "com.microsoft.VSCode",
    "org.mozilla.firefox",
    "", // empty bundle id -- exercises the init-cache quirk
    "com.apple.Terminal",
];

#[derive(Debug, Clone)]
enum Op {
    Get { bundle: usize, current: i32 },
    Set { bundle: usize, value: i32 },
}

fn op_strategy() -> impl Strategy<Value = Op> {
    prop_oneof![
        (0usize..BUNDLES.len(), -2i32..4).prop_map(|(bundle, current)| Op::Get { bundle, current }),
        (0usize..BUNDLES.len(), -2i32..4).prop_map(|(bundle, value)| Op::Set { bundle, value }),
    ]
}

fn config(default_cases: u32) -> ProptestConfig {
    let cases = std::env::var("PROPTEST_CASES")
        .ok()
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(default_cases);
    ProptestConfig::with_cases(cases)
}

#[test]
fn smart_blob_round_trip_matches_oracle() {
    // Build a table, serialize, and confirm the oracle re-serializes the same
    // bytes -- proving the on-disk blob format is byte-exact with OpenKey.
    let mut t = SmartSwitchTable::new();
    t.set_status("com.apple.Safari", 1);
    t.set_status("com.microsoft.VSCode", 0);
    t.set_status("org.mozilla.firefox", 3);
    t.set_status("z.very.long.bundle.identifier.example.app", -1);
    let blob = t.to_blob();

    let opts = OkOptions::default();
    let mut session = Oracle::lock(&opts);
    session.set_smart_blob(&blob);
    let oracle_blob = session.get_smart_blob(4096);
    assert_eq!(blob, oracle_blob, "smart-switch blob must be byte-exact");

    // ... and Rust re-parses its own blob identically.
    let t2 = SmartSwitchTable::from_blob(&blob);
    assert_eq!(blob, t2.to_blob());
}

proptest! {
    #![proptest_config(config(3000))]

    #[test]
    fn smart_switch_get_set_sequence_matches_oracle(ops in prop::collection::vec(op_strategy(), 1..=40)) {
        let opts = OkOptions::default();
        let mut session = Oracle::lock(&opts);
        let mut table = SmartSwitchTable::new();

        for (step, op) in ops.iter().enumerate() {
            match *op {
                Op::Get { bundle, current } => {
                    let b = BUNDLES[bundle];
                    let rust = table.get_status(b, current);
                    let oracle = session.smart_get(b, current);
                    prop_assert_eq!(
                        rust, oracle,
                        "get_status mismatch at step {} bundle={:?} current={}",
                        step, b, current
                    );
                }
                Op::Set { bundle, value } => {
                    let b = BUNDLES[bundle];
                    table.set_status(b, value);
                    session.smart_set(b, value);
                }
            }
        }

        // Final on-disk state must agree byte-for-byte.
        let oracle_blob = session.get_smart_blob(8192);
        prop_assert_eq!(table.to_blob(), oracle_blob, "final blob mismatch\n  ops={:?}", ops);
    }
}

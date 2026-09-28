// Test file to demonstrate weighted sequencer functionality
use crate::models::sequencer::SequencerSet;
use std::collections::HashMap;

#[test]
fn test_sequencer_stakes_are_tracked() {
    let mut stakes = HashMap::new();
    stakes.insert("peer1".to_string(), 10);
    stakes.insert("peer2".to_string(), 5);
    stakes.insert("peer3".to_string(), 1);

    let nominated = vec![
        "peer1".to_string(),
        "peer2".to_string(),
        "peer3".to_string(),
    ];

    let sequencer_set =
        SequencerSet::new_with_stakes(1, 2, nominated, Vec::new(), Vec::new(), stakes);

    assert_eq!(sequencer_set.get_sequencer_stake("peer1"), 10);
    assert_eq!(sequencer_set.get_sequencer_stake("peer2"), 5);
    assert_eq!(sequencer_set.get_sequencer_stake("peer3"), 1);
    assert_eq!(sequencer_set.get_sequencer_stake("unknown"), 1); // Default
}

#[test]
fn test_get_active_sequencers_with_stakes() {
    let mut stakes = HashMap::new();
    stakes.insert("peer1".to_string(), 20);
    stakes.insert("peer2".to_string(), 15);
    stakes.insert("peer3".to_string(), 3);

    let nominated = vec!["peer1".to_string(), "peer2".to_string()];
    let staked = vec!["peer3".to_string()];

    let sequencer_set = SequencerSet::new_with_stakes(1, 2, nominated, staked, Vec::new(), stakes);

    let active_with_stakes = sequencer_set.get_active_sequencers_with_stakes();

    assert_eq!(active_with_stakes.len(), 3);
    assert_eq!(active_with_stakes[0], ("peer1".to_string(), 20));
    assert_eq!(active_with_stakes[1], ("peer2".to_string(), 15));
    assert_eq!(active_with_stakes[2], ("peer3".to_string(), 3));

    // Total stake should be 38
    let total_stake: u64 = active_with_stakes.iter().map(|(_, s)| s).sum();
    assert_eq!(total_stake, 38);
}

#[test]
fn test_single_sequencer_high_stake() {
    // Simulates a single sequencer nominated in all 40 blocks of an epoch
    let mut stakes = HashMap::new();
    stakes.insert("peer1".to_string(), 40);

    let nominated = vec!["peer1".to_string()];

    let sequencer_set =
        SequencerSet::new_with_stakes(1, 2, nominated, Vec::new(), Vec::new(), stakes);

    assert_eq!(sequencer_set.get_sequencer_stake("peer1"), 40);

    let active = sequencer_set.get_active_sequencers_with_stakes();
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].1, 40); // Stake of 40
}

#[test]
fn test_multiple_sequencers_different_stakes() {
    // Simulates 3 sequencers with different nomination counts
    let mut stakes = HashMap::new();
    stakes.insert("peer1".to_string(), 20); // Nominated in 20 blocks
    stakes.insert("peer2".to_string(), 15); // Nominated in 15 blocks
    stakes.insert("peer3".to_string(), 5); // Nominated in 5 blocks

    let nominated = vec![
        "peer1".to_string(),
        "peer2".to_string(),
        "peer3".to_string(),
    ];

    let _sequencer_set =
        SequencerSet::new_with_stakes(1, 2, nominated, Vec::new(), Vec::new(), stakes);

    // Verify proportional voting power
    let total_stake = 20 + 15 + 5; // 40
    let peer1_percentage = (20.0 / total_stake as f64) * 100.0;
    let peer2_percentage = (15.0 / total_stake as f64) * 100.0;
    let peer3_percentage = (5.0 / total_stake as f64) * 100.0;

    assert_eq!(peer1_percentage, 50.0); // 50% voting power
    assert_eq!(peer2_percentage, 37.5); // 37.5% voting power
    assert_eq!(peer3_percentage, 12.5); // 12.5% voting power
}

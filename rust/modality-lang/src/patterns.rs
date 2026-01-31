//! Common Contract Patterns
//!
//! This module provides reusable patterns for real-world agent contracts:
//! - Timeout/expiry handling
//! - Dispute resolution
//! - Escrow with protection
//! - Multi-stage agreements
//!
//! These patterns can be composed to build complex contracts.

use crate::ast::{Model, Part, Transition, Property, PropertySign};

/// Generate a dispute resolution flow
///
/// Adds states: disputed → arbitration → resolved_*
/// Requires: arbitrator party
pub fn add_dispute_resolution(
    model: &mut Model,
    part_name: &str,
    from_state: &str,
    parties: &[&str],
    arbitrator: &str,
) {
    let part = model.parts.iter_mut()
        .find(|p| p.name == part_name)
        .expect("Part not found");

    // Any party can dispute
    for party in parties {
        let mut t = Transition::new(from_state.to_string(), "disputed".to_string());
        t.add_property(Property::new(PropertySign::Plus, "DISPUTE".to_string()));
        t.add_property(Property::new(PropertySign::Plus, format!("SIGNED_BY_{}", party.to_uppercase())));
        part.add_transition(t);
    }

    // Request arbitration
    let mut t = Transition::new("disputed".to_string(), "arbitration".to_string());
    t.add_property(Property::new(PropertySign::Plus, "REQUEST_ARBITRATION".to_string()));
    part.add_transition(t);

    // Arbitrator rulings
    for (outcome, next_state) in &[("FAVOR_A", "resolved_a"), ("FAVOR_B", "resolved_b"), ("SPLIT", "resolved_split")] {
        let mut t = Transition::new("arbitration".to_string(), next_state.to_string());
        t.add_property(Property::new(PropertySign::Plus, format!("RULING_{}", outcome)));
        t.add_property(Property::new(PropertySign::Plus, format!("SIGNED_BY_{}", arbitrator.to_uppercase())));
        part.add_transition(t);
    }
}

/// Generate a timeout flow
///
/// Allows transition if a timeout has elapsed
pub fn add_timeout_transition(
    model: &mut Model,
    part_name: &str,
    from_state: &str,
    to_state: &str,
    timeout_name: &str,
) {
    let part = model.parts.iter_mut()
        .find(|p| p.name == part_name)
        .expect("Part not found");

    let mut t = Transition::new(from_state.to_string(), to_state.to_string());
    t.add_property(Property::new(PropertySign::Plus, format!("{}_ELAPSED", timeout_name.to_uppercase())));
    part.add_transition(t);
}

/// Generate a cancellation flow with refund
pub fn add_cancellation(
    model: &mut Model,
    part_name: &str,
    cancellable_states: &[&str],
    parties_required: &[&str],  // All must sign to cancel
) {
    let part = model.parts.iter_mut()
        .find(|p| p.name == part_name)
        .expect("Part not found");

    for state in cancellable_states {
        let mut t = Transition::new(state.to_string(), "cancelled".to_string());
        t.add_property(Property::new(PropertySign::Plus, "CANCEL".to_string()));
        for party in parties_required {
            t.add_property(Property::new(PropertySign::Plus, format!("SIGNED_BY_{}", party.to_uppercase())));
        }
        part.add_transition(t);
    }

    // Cancelled → refund → complete
    let mut t = Transition::new("cancelled".to_string(), "refunded".to_string());
    t.add_property(Property::new(PropertySign::Plus, "REFUND".to_string()));
    part.add_transition(t);

    let t = Transition::new("refunded".to_string(), "complete".to_string());
    part.add_transition(t);
}

/// Escrow with full protections
///
/// Includes: deposit, delivery, release, dispute, timeout, cancellation
pub fn escrow_protected(depositor: &str, deliverer: &str, arbitrator: &str) -> Model {
    let mut model = Model::new("ProtectedEscrow".to_string());
    let mut part = Part::new("escrow".to_string());

    let signer_depositor = format!("SIGNED_BY_{}", depositor.to_uppercase());
    let signer_deliverer = format!("SIGNED_BY_{}", deliverer.to_uppercase());

    // Happy path: init → deposited → delivered → released → complete
    let mut t1 = Transition::new("init".to_string(), "deposited".to_string());
    t1.add_property(Property::new(PropertySign::Plus, "DEPOSIT".to_string()));
    t1.add_property(Property::new(PropertySign::Plus, signer_depositor.clone()));
    part.add_transition(t1);

    let mut t2 = Transition::new("deposited".to_string(), "delivered".to_string());
    t2.add_property(Property::new(PropertySign::Plus, "DELIVER".to_string()));
    t2.add_property(Property::new(PropertySign::Plus, signer_deliverer.clone()));
    part.add_transition(t2);

    let mut t3 = Transition::new("delivered".to_string(), "released".to_string());
    t3.add_property(Property::new(PropertySign::Plus, "RELEASE".to_string()));
    t3.add_property(Property::new(PropertySign::Plus, signer_depositor.clone()));
    part.add_transition(t3);

    part.add_transition(Transition::new("released".to_string(), "complete".to_string()));

    // Timeout: if deliverer doesn't deliver in time, depositor can reclaim
    let mut t_timeout = Transition::new("deposited".to_string(), "timeout_refund".to_string());
    t_timeout.add_property(Property::new(PropertySign::Plus, "DELIVERY_TIMEOUT_ELAPSED".to_string()));
    t_timeout.add_property(Property::new(PropertySign::Plus, signer_depositor.clone()));
    part.add_transition(t_timeout);

    let mut t_reclaim = Transition::new("timeout_refund".to_string(), "complete".to_string());
    t_reclaim.add_property(Property::new(PropertySign::Plus, "RECLAIM".to_string()));
    part.add_transition(t_reclaim);

    // Dispute from delivered state
    let mut t_dispute = Transition::new("delivered".to_string(), "disputed".to_string());
    t_dispute.add_property(Property::new(PropertySign::Plus, "DISPUTE".to_string()));
    t_dispute.add_property(Property::new(PropertySign::Plus, signer_depositor.clone()));
    part.add_transition(t_dispute);

    // Arbitration
    let mut t_arb = Transition::new("disputed".to_string(), "arbitration".to_string());
    t_arb.add_property(Property::new(PropertySign::Plus, "REQUEST_ARBITRATION".to_string()));
    part.add_transition(t_arb);

    let signer_arbitrator = format!("SIGNED_BY_{}", arbitrator.to_uppercase());

    // Arbitrator outcomes
    let mut t_favor_depositor = Transition::new("arbitration".to_string(), "refund_depositor".to_string());
    t_favor_depositor.add_property(Property::new(PropertySign::Plus, "RULING_FAVOR_DEPOSITOR".to_string()));
    t_favor_depositor.add_property(Property::new(PropertySign::Plus, signer_arbitrator.clone()));
    part.add_transition(t_favor_depositor);

    let mut t_favor_deliverer = Transition::new("arbitration".to_string(), "release_to_deliverer".to_string());
    t_favor_deliverer.add_property(Property::new(PropertySign::Plus, "RULING_FAVOR_DELIVERER".to_string()));
    t_favor_deliverer.add_property(Property::new(PropertySign::Plus, signer_arbitrator.clone()));
    part.add_transition(t_favor_deliverer);

    let mut t_split = Transition::new("arbitration".to_string(), "split_funds".to_string());
    t_split.add_property(Property::new(PropertySign::Plus, "RULING_SPLIT".to_string()));
    t_split.add_property(Property::new(PropertySign::Plus, signer_arbitrator.clone()));
    part.add_transition(t_split);

    // Resolution states → complete
    part.add_transition(Transition::new("refund_depositor".to_string(), "complete".to_string()));
    part.add_transition(Transition::new("release_to_deliverer".to_string(), "complete".to_string()));
    part.add_transition(Transition::new("split_funds".to_string(), "complete".to_string()));

    // Cancellation (mutual) from deposited state
    let mut t_cancel = Transition::new("deposited".to_string(), "cancelled".to_string());
    t_cancel.add_property(Property::new(PropertySign::Plus, "CANCEL".to_string()));
    t_cancel.add_property(Property::new(PropertySign::Plus, signer_depositor.clone()));
    t_cancel.add_property(Property::new(PropertySign::Plus, signer_deliverer.clone()));
    part.add_transition(t_cancel);

    part.add_transition(Transition::new("cancelled".to_string(), "complete".to_string()));

    model.add_part(part);
    model
}

/// Multi-stage milestone contract
///
/// Work is divided into milestones, each requiring delivery and confirmation
pub fn milestone_contract(client: &str, contractor: &str, milestones: usize) -> Model {
    let mut model = Model::new("MilestoneContract".to_string());
    let mut part = Part::new("milestones".to_string());

    let signer_client = format!("SIGNED_BY_{}", client.to_uppercase());
    let signer_contractor = format!("SIGNED_BY_{}", contractor.to_uppercase());

    // Agreement phase
    let mut t_agree = Transition::new("init".to_string(), "agreed".to_string());
    t_agree.add_property(Property::new(PropertySign::Plus, "AGREE".to_string()));
    t_agree.add_property(Property::new(PropertySign::Plus, signer_client.clone()));
    t_agree.add_property(Property::new(PropertySign::Plus, signer_contractor.clone()));
    part.add_transition(t_agree);

    // First milestone deposit
    let mut t_deposit = Transition::new("agreed".to_string(), "milestone_1_funded".to_string());
    t_deposit.add_property(Property::new(PropertySign::Plus, "DEPOSIT_MILESTONE_1".to_string()));
    t_deposit.add_property(Property::new(PropertySign::Plus, signer_client.clone()));
    part.add_transition(t_deposit);

    // Each milestone: funded → delivered → confirmed → (next milestone or complete)
    for m in 1..=milestones {
        let funded = format!("milestone_{}_funded", m);
        let delivered = format!("milestone_{}_delivered", m);
        let confirmed = format!("milestone_{}_confirmed", m);

        // Contractor delivers
        let mut t_deliver = Transition::new(funded.clone(), delivered.clone());
        t_deliver.add_property(Property::new(PropertySign::Plus, format!("DELIVER_MILESTONE_{}", m)));
        t_deliver.add_property(Property::new(PropertySign::Plus, signer_contractor.clone()));
        part.add_transition(t_deliver);

        // Client confirms
        let mut t_confirm = Transition::new(delivered.clone(), confirmed.clone());
        t_confirm.add_property(Property::new(PropertySign::Plus, format!("CONFIRM_MILESTONE_{}", m)));
        t_confirm.add_property(Property::new(PropertySign::Plus, signer_client.clone()));
        part.add_transition(t_confirm);

        // After confirmation: fund next milestone or complete
        if m < milestones {
            let next_funded = format!("milestone_{}_funded", m + 1);
            let mut t_fund_next = Transition::new(confirmed.clone(), next_funded);
            t_fund_next.add_property(Property::new(PropertySign::Plus, format!("DEPOSIT_MILESTONE_{}", m + 1)));
            t_fund_next.add_property(Property::new(PropertySign::Plus, signer_client.clone()));
            part.add_transition(t_fund_next);
        } else {
            let t_complete = Transition::new(confirmed, "complete".to_string());
            part.add_transition(t_complete);
        }
    }

    model.add_part(part);
    model
}

/// Recurring payment contract
///
/// Automatic payments at intervals, with ability to pause/cancel
pub fn recurring_payment(payer: &str, recipient: &str) -> Model {
    let mut model = Model::new("RecurringPayment".to_string());
    let mut part = Part::new("subscription".to_string());

    let signer_payer = format!("SIGNED_BY_{}", payer.to_uppercase());
    let signer_recipient = format!("SIGNED_BY_{}", recipient.to_uppercase());

    // Setup: init → active
    let mut t_activate = Transition::new("init".to_string(), "active".to_string());
    t_activate.add_property(Property::new(PropertySign::Plus, "ACTIVATE".to_string()));
    t_activate.add_property(Property::new(PropertySign::Plus, signer_payer.clone()));
    t_activate.add_property(Property::new(PropertySign::Plus, signer_recipient.clone()));
    part.add_transition(t_activate);

    // Active: payments cycle
    let mut t_pay = Transition::new("active".to_string(), "payment_due".to_string());
    t_pay.add_property(Property::new(PropertySign::Plus, "PERIOD_ELAPSED".to_string()));
    part.add_transition(t_pay);

    let mut t_paid = Transition::new("payment_due".to_string(), "active".to_string());
    t_paid.add_property(Property::new(PropertySign::Plus, "PAY".to_string()));
    t_paid.add_property(Property::new(PropertySign::Plus, signer_payer.clone()));
    part.add_transition(t_paid);

    // Pause
    let mut t_pause = Transition::new("active".to_string(), "paused".to_string());
    t_pause.add_property(Property::new(PropertySign::Plus, "PAUSE".to_string()));
    t_pause.add_property(Property::new(PropertySign::Plus, signer_payer.clone()));
    part.add_transition(t_pause);

    let mut t_resume = Transition::new("paused".to_string(), "active".to_string());
    t_resume.add_property(Property::new(PropertySign::Plus, "RESUME".to_string()));
    t_resume.add_property(Property::new(PropertySign::Plus, signer_payer.clone()));
    part.add_transition(t_resume);

    // Cancel
    let mut t_cancel = Transition::new("active".to_string(), "cancelled".to_string());
    t_cancel.add_property(Property::new(PropertySign::Plus, "CANCEL".to_string()));
    t_cancel.add_property(Property::new(PropertySign::Plus, signer_payer.clone()));
    part.add_transition(t_cancel);

    let mut t_cancel_paused = Transition::new("paused".to_string(), "cancelled".to_string());
    t_cancel_paused.add_property(Property::new(PropertySign::Plus, "CANCEL".to_string()));
    t_cancel_paused.add_property(Property::new(PropertySign::Plus, signer_payer.clone()));
    part.add_transition(t_cancel_paused);

    // Payment timeout → suspended
    let mut t_timeout = Transition::new("payment_due".to_string(), "suspended".to_string());
    t_timeout.add_property(Property::new(PropertySign::Plus, "PAYMENT_TIMEOUT_ELAPSED".to_string()));
    part.add_transition(t_timeout);

    // Recover from suspension
    let mut t_recover = Transition::new("suspended".to_string(), "active".to_string());
    t_recover.add_property(Property::new(PropertySign::Plus, "PAY_OVERDUE".to_string()));
    t_recover.add_property(Property::new(PropertySign::Plus, signer_payer.clone()));
    part.add_transition(t_recover);

    model.add_part(part);
    model
}

/// Auction contract
///
/// Multiple bidders, highest bid wins
pub fn auction(seller: &str, min_bidders: usize) -> Model {
    let mut model = Model::new("Auction".to_string());
    let mut part = Part::new("bidding".to_string());

    let signer_seller = format!("SIGNED_BY_{}", seller.to_uppercase());

    // Start auction
    let mut t_start = Transition::new("init".to_string(), "open".to_string());
    t_start.add_property(Property::new(PropertySign::Plus, "START_AUCTION".to_string()));
    t_start.add_property(Property::new(PropertySign::Plus, signer_seller.clone()));
    part.add_transition(t_start);

    // Bidding phase (any number of bids)
    let mut t_bid = Transition::new("open".to_string(), "open".to_string());
    t_bid.add_property(Property::new(PropertySign::Plus, "BID".to_string()));
    t_bid.add_property(Property::new(PropertySign::Plus, "HIGHER_THAN_CURRENT".to_string()));
    part.add_transition(t_bid);

    // Close bidding
    let mut t_close = Transition::new("open".to_string(), "closed".to_string());
    t_close.add_property(Property::new(PropertySign::Plus, "CLOSE_BIDDING".to_string()));
    t_close.add_property(Property::new(PropertySign::Plus, signer_seller.clone()));
    part.add_transition(t_close);

    // Also allow auto-close via timeout
    let mut t_timeout_close = Transition::new("open".to_string(), "closed".to_string());
    t_timeout_close.add_property(Property::new(PropertySign::Plus, "AUCTION_TIMEOUT_ELAPSED".to_string()));
    part.add_transition(t_timeout_close);

    // Determine winner
    let mut t_winner = Transition::new("closed".to_string(), "winner_selected".to_string());
    t_winner.add_property(Property::new(PropertySign::Plus, "SELECT_WINNER".to_string()));
    t_winner.add_property(Property::new(PropertySign::Plus, format!("MIN_BIDDERS_{}", min_bidders)));
    part.add_transition(t_winner);

    // No winner (not enough bidders)
    let mut t_no_winner = Transition::new("closed".to_string(), "no_sale".to_string());
    t_no_winner.add_property(Property::new(PropertySign::Plus, "INSUFFICIENT_BIDDERS".to_string()));
    part.add_transition(t_no_winner);

    // Winner pays
    let mut t_pay = Transition::new("winner_selected".to_string(), "paid".to_string());
    t_pay.add_property(Property::new(PropertySign::Plus, "PAY".to_string()));
    t_pay.add_property(Property::new(PropertySign::Plus, "SIGNED_BY_WINNER".to_string()));
    part.add_transition(t_pay);

    // Seller transfers
    let mut t_transfer = Transition::new("paid".to_string(), "complete".to_string());
    t_transfer.add_property(Property::new(PropertySign::Plus, "TRANSFER".to_string()));
    t_transfer.add_property(Property::new(PropertySign::Plus, signer_seller.clone()));
    part.add_transition(t_transfer);

    // No sale ends
    part.add_transition(Transition::new("no_sale".to_string(), "complete".to_string()));

    model.add_part(part);
    model
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escrow_protected() {
        let model = escrow_protected("Alice", "Bob", "Arbitrator");
        assert_eq!(model.name, "ProtectedEscrow");
        assert_eq!(model.parts.len(), 1);
        
        let part = &model.parts[0];
        // Should have many transitions for all paths
        assert!(part.transitions.len() >= 10);
    }

    #[test]
    fn test_milestone_contract() {
        let model = milestone_contract("Client", "Contractor", 3);
        assert_eq!(model.name, "MilestoneContract");
        
        let part = &model.parts[0];
        // Agreement + 3 milestones * 3 transitions each + final
        assert!(part.transitions.len() >= 10);
    }

    #[test]
    fn test_recurring_payment() {
        let model = recurring_payment("Payer", "Recipient");
        assert_eq!(model.name, "RecurringPayment");
        
        let part = &model.parts[0];
        assert!(part.transitions.len() >= 8);
    }

    #[test]
    fn test_auction() {
        let model = auction("Seller", 2);
        assert_eq!(model.name, "Auction");
        
        let part = &model.parts[0];
        assert!(part.transitions.len() >= 8);
    }
}

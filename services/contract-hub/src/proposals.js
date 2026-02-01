/**
 * Proposal Commits - Threshold signature support
 * 
 * Commits that require N-of-M signatures before finalization.
 */

import { createHash } from 'crypto';

/**
 * Proposal states
 */
export const ProposalStatus = {
  PENDING: 'pending',
  FINALIZED: 'finalized',
  CANCELLED: 'cancelled',
  EXPIRED: 'expired'
};

/**
 * Create a new proposal
 */
export function createProposal({
  payload,
  thresholdRequired,
  signers,
  expiresAt,
  proposedBy
}) {
  const id = 'prop_' + createHash('sha256')
    .update(JSON.stringify(payload) + Date.now() + Math.random())
    .digest('hex')
    .slice(0, 16);
  
  return {
    id,
    payload,
    threshold: {
      required: thresholdRequired,
      signers
    },
    proposed_by: proposedBy,
    proposed_at: Date.now(),
    expires_at: expiresAt || null,
    status: ProposalStatus.PENDING,
    approvals: {}
  };
}

/**
 * Add an approval to a proposal
 * Returns { success, threshold_met, error }
 */
export function addApproval(proposal, signerId, signature) {
  // Check proposal is pending
  if (proposal.status !== ProposalStatus.PENDING) {
    return { success: false, error: `Proposal is ${proposal.status}` };
  }
  
  // Check expiration
  if (proposal.expires_at && Date.now() > proposal.expires_at) {
    proposal.status = ProposalStatus.EXPIRED;
    return { success: false, error: 'Proposal has expired' };
  }
  
  // Check signer is authorized
  if (!proposal.threshold.signers.includes(signerId)) {
    return { success: false, error: `Signer ${signerId} not in authorized signers` };
  }
  
  // Check for duplicate
  if (proposal.approvals[signerId]) {
    return { success: false, error: `${signerId} has already approved` };
  }
  
  // Add approval
  proposal.approvals[signerId] = {
    signature,
    approved_at: Date.now()
  };
  
  // Check if threshold met
  const approvalCount = Object.keys(proposal.approvals).length;
  const thresholdMet = approvalCount >= proposal.threshold.required;
  
  return { 
    success: true, 
    threshold_met: thresholdMet,
    approval_count: approvalCount,
    required: proposal.threshold.required
  };
}

/**
 * Finalize a proposal (mark as complete, return payload for execution)
 */
export function finalizeProposal(proposal) {
  if (proposal.status !== ProposalStatus.PENDING) {
    return { success: false, error: `Proposal is ${proposal.status}` };
  }
  
  const approvalCount = Object.keys(proposal.approvals).length;
  if (approvalCount < proposal.threshold.required) {
    return { 
      success: false, 
      error: `Threshold not met (${approvalCount}/${proposal.threshold.required})` 
    };
  }
  
  proposal.status = ProposalStatus.FINALIZED;
  proposal.finalized_at = Date.now();
  
  return {
    success: true,
    payload: proposal.payload,
    signers: Object.keys(proposal.approvals)
  };
}

/**
 * Cancel a proposal
 */
export function cancelProposal(proposal, cancellerId) {
  if (proposal.status !== ProposalStatus.PENDING) {
    return { success: false, error: `Proposal is ${proposal.status}` };
  }
  
  // Only proposer can cancel (for now)
  if (cancellerId !== proposal.proposed_by) {
    return { success: false, error: 'Only proposer can cancel' };
  }
  
  proposal.status = ProposalStatus.CANCELLED;
  proposal.cancelled_at = Date.now();
  proposal.cancelled_by = cancellerId;
  
  return { success: true };
}

/**
 * Check and expire old proposals
 */
export function expireProposals(proposals) {
  const now = Date.now();
  const expired = [];
  
  for (const proposal of proposals) {
    if (proposal.status === ProposalStatus.PENDING && 
        proposal.expires_at && 
        now > proposal.expires_at) {
      proposal.status = ProposalStatus.EXPIRED;
      expired.push(proposal.id);
    }
  }
  
  return expired;
}

/**
 * Validate proposal payload against contract model
 * (To be integrated with contract-validator.js)
 */
export async function validateProposalPayload(validator, payload) {
  // Pre-validate the payload
  if (payload.method === 'ACTION') {
    const result = validator.validateAction(payload.action, payload);
    return result;
  }
  
  // Other methods (POST, RULE) are generally valid
  return { ok: true };
}

export default {
  ProposalStatus,
  createProposal,
  addApproval,
  finalizeProposal,
  cancelProposal,
  expireProposals,
  validateProposalPayload
};

/** Intent template patterns for common contract types */

export interface IntentTemplate {
  '@atp_intent': '1.0';
  pattern: string;
  parties: Record<string, string>;
  terms: Record<string, unknown>;
  [key: string]: unknown;
}

export interface EscrowTerms {
  amount: number;
  currency: string;
  delivery_deadline: string;
  dispute_window_hours: number;
  [key: string]: unknown;
}

export interface TaskDelegationTerms {
  task: string;
  payment: number;
  deadline: string;
  [key: string]: unknown;
}

export interface DataExchangeTerms {
  data_description: string;
  price: number;
  currency: string;
  format: string;
  [key: string]: unknown;
}

export function escrow(
  parties: { buyer: string; seller: string; arbiter?: string },
  terms: EscrowTerms,
  options?: {
    buyer_protections?: string[];
    seller_protections?: string[];
  },
): IntentTemplate {
  return {
    '@atp_intent': '1.0',
    pattern: 'escrow',
    parties,
    terms: terms as Record<string, unknown>,
    buyer_protections: options?.buyer_protections ?? [
      'delivery_required',
      'dispute_allowed',
    ],
    seller_protections: options?.seller_protections ?? [
      'payment_guaranteed',
      'no_clawback_after_release',
    ],
  };
}

export function taskDelegation(
  parties: { delegator: string; worker: string },
  terms: TaskDelegationTerms,
  options?: {
    delegator_protections?: string[];
    worker_protections?: string[];
  },
): IntentTemplate {
  return {
    '@atp_intent': '1.0',
    pattern: 'task_delegation',
    parties,
    terms: terms as Record<string, unknown>,
    delegator_protections: options?.delegator_protections ?? [
      'payment_on_completion',
      'quality_review_period',
    ],
    worker_protections: options?.worker_protections ?? [
      'payment_guaranteed_if_accepted',
      'clear_requirements',
    ],
  };
}

export function dataExchange(
  parties: { provider: string; consumer: string },
  terms: DataExchangeTerms,
  options?: {
    provider_protections?: string[];
    consumer_protections?: string[];
  },
): IntentTemplate {
  return {
    '@atp_intent': '1.0',
    pattern: 'data_exchange',
    parties,
    terms: terms as Record<string, unknown>,
    provider_protections: options?.provider_protections ?? [
      'payment_before_delivery',
    ],
    consumer_protections: options?.consumer_protections ?? [
      'data_quality_guaranteed',
      'refund_if_invalid',
    ],
  };
}

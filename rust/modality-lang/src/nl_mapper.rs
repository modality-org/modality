//! Natural Language to Pattern Mapping
//!
//! Maps natural language descriptions to contract templates.
//! This provides a simple keyword-based mapping that can be extended
//! with LLM integration for more sophisticated understanding.

use crate::ast::Model;
use crate::synthesis::templates;

/// Result of NL mapping attempt
#[derive(Debug, Clone)]
pub struct NLMappingResult {
    /// The identified pattern
    pub pattern: ContractPattern,
    /// Extracted parties from the description
    pub parties: Vec<String>,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f64,
    /// Generated model (if successful)
    pub model: Option<Model>,
    /// Suggestions or clarifying questions
    pub suggestions: Vec<String>,
}

/// Recognized contract patterns
#[derive(Debug, Clone, PartialEq)]
pub enum ContractPattern {
    Escrow,
    Handshake,
    MutualCooperation,
    AtomicSwap,
    Multisig,
    ServiceAgreement,
    Delegation,
    Auction,
    Subscription,
    Milestone,
    Unknown,
}

impl ContractPattern {
    pub fn name(&self) -> &'static str {
        match self {
            ContractPattern::Escrow => "escrow",
            ContractPattern::Handshake => "handshake",
            ContractPattern::MutualCooperation => "mutual_cooperation",
            ContractPattern::AtomicSwap => "atomic_swap",
            ContractPattern::Multisig => "multisig",
            ContractPattern::ServiceAgreement => "service_agreement",
            ContractPattern::Delegation => "delegation",
            ContractPattern::Auction => "auction",
            ContractPattern::Subscription => "subscription",
            ContractPattern::Milestone => "milestone",
            ContractPattern::Unknown => "unknown",
        }
    }
}

/// Keywords that suggest each pattern
struct PatternKeywords {
    pattern: ContractPattern,
    keywords: Vec<&'static str>,
    weight: f64,
}

fn get_pattern_keywords() -> Vec<PatternKeywords> {
    vec![
        PatternKeywords {
            pattern: ContractPattern::Escrow,
            keywords: vec![
                "escrow", "hold funds", "release payment", "deposit", 
                "third party holds", "conditional release", "buyer seller",
                "deliver then pay", "payment protection", "funds held",
                "secure payment", "goods delivered", "release funds",
                "payment on delivery", "delivery confirmed"
            ],
            weight: 1.0,
        },
        PatternKeywords {
            pattern: ContractPattern::Handshake,
            keywords: vec![
                "handshake", "both sign", "both must sign", "mutual agreement", 
                "both parties agree", "both parties must", "two signatures", 
                "joint commitment", "bilateral", "two party agreement"
            ],
            weight: 1.0,
        },
        PatternKeywords {
            pattern: ContractPattern::MutualCooperation,
            keywords: vec![
                "cooperation", "no defection", "both cooperate", "prisoner",
                "tit for tat", "mutual benefit", "neither can defect"
            ],
            weight: 1.0,
        },
        PatternKeywords {
            pattern: ContractPattern::AtomicSwap,
            keywords: vec![
                "atomic swap", "swap", "exchange", "trade",
                "both commit", "simultaneous", "cross-chain",
                "trustless exchange", "token swap", "crypto swap",
                "asset exchange", "peer to peer exchange"
            ],
            weight: 1.0,
        },
        PatternKeywords {
            pattern: ContractPattern::Multisig,
            keywords: vec![
                "multisig", "multi-signature", "n of m", "2 of 3",
                "multiple signatures", "quorum", "threshold signature"
            ],
            weight: 1.0,
        },
        PatternKeywords {
            pattern: ContractPattern::ServiceAgreement,
            keywords: vec![
                "service", "offer accept", "deliver confirm",
                "provider consumer", "work for payment", "contract work",
                "freelance", "gig", "job completion", "task payment",
                "service rendered", "work delivered", "invoice"
            ],
            weight: 1.0,
        },
        PatternKeywords {
            pattern: ContractPattern::Delegation,
            keywords: vec![
                "delegate", "delegation", "authorize", "on behalf",
                "proxy", "agent authority", "grant permission", "revoke",
                "power of attorney", "representative", "empowerment",
                "act for me", "signing authority", "delegated access"
            ],
            weight: 1.0,
        },
        PatternKeywords {
            pattern: ContractPattern::Auction,
            keywords: vec![
                "auction", "bid", "bidding", "highest bidder",
                "sell to highest", "listing", "winner pays"
            ],
            weight: 1.0,
        },
        PatternKeywords {
            pattern: ContractPattern::Subscription,
            keywords: vec![
                "subscription", "recurring", "monthly", "annual",
                "renew", "cancel", "access period", "membership"
            ],
            weight: 1.0,
        },
        PatternKeywords {
            pattern: ContractPattern::Milestone,
            keywords: vec![
                "milestone", "phase", "stage", "deliverable",
                "partial payment", "progress payment", "project phase"
            ],
            weight: 1.0,
        },
    ]
}

/// Extract potential party names from description
fn extract_parties(description: &str) -> Vec<String> {
    let mut parties = Vec::new();
    let lower = description.to_lowercase();
    
    // Common party name patterns (order matters - more specific first)
    let party_patterns = [
        // Specific roles
        ("service provider", "ServiceProvider"),
        ("service consumer", "ServiceConsumer"),
        ("party a", "PartyA"),
        ("party b", "PartyB"),
        ("first party", "FirstParty"),
        ("second party", "SecondParty"),
        // Generic roles
        ("buyer", "Buyer"),
        ("seller", "Seller"),
        ("provider", "Provider"),
        ("consumer", "Consumer"),
        ("client", "Client"),
        ("contractor", "Contractor"),
        ("principal", "Principal"),
        ("agent", "Agent"),
        ("depositor", "Depositor"),
        ("deliverer", "Deliverer"),
        ("recipient", "Recipient"),
        ("sender", "Sender"),
        ("receiver", "Receiver"),
        ("bidder", "Bidder"),
        ("subscriber", "Subscriber"),
        ("moderator", "Moderator"),
        ("admin", "Admin"),
        ("owner", "Owner"),
        ("user", "User"),
        ("vendor", "Vendor"),
        ("merchant", "Merchant"),
        ("customer", "Customer"),
        ("employee", "Employee"),
        ("employer", "Employer"),
        ("tenant", "Tenant"),
        ("landlord", "Landlord"),
        // Common names
        ("alice", "Alice"),
        ("bob", "Bob"),
        ("carol", "Carol"),
        ("dave", "Dave"),
        ("eve", "Eve"),
        ("frank", "Frank"),
    ];
    
    for (pattern, name) in party_patterns {
        if lower.contains(pattern) && !parties.contains(&name.to_string()) {
            parties.push(name.to_string());
        }
    }
    
    // If no parties found, use defaults
    if parties.is_empty() {
        parties.push("PartyA".to_string());
        parties.push("PartyB".to_string());
    }
    
    parties
}

/// Map natural language description to contract pattern
pub fn map_nl_to_pattern(description: &str) -> NLMappingResult {
    let lower = description.to_lowercase();
    let patterns = get_pattern_keywords();
    
    // Score each pattern based on keyword matches
    let mut best_pattern = ContractPattern::Unknown;
    let mut best_score = 0.0;
    let mut total_matches = 0;
    
    for pk in &patterns {
        let mut score = 0.0;
        for keyword in &pk.keywords {
            if lower.contains(keyword) {
                score += pk.weight;
                total_matches += 1;
            }
        }
        
        if score > best_score {
            best_score = score;
            best_pattern = pk.pattern.clone();
        }
    }
    
    // Calculate confidence
    let confidence = if total_matches > 0 {
        (best_score / (total_matches as f64 * 0.5)).min(1.0)
    } else {
        0.0
    };
    
    // Extract parties
    let parties = extract_parties(description);
    
    // Generate model if pattern is known
    let model = generate_model(&best_pattern, &parties);
    
    // Generate suggestions
    let suggestions = if best_pattern == ContractPattern::Unknown {
        vec![
            "Try describing the contract using terms like: escrow, handshake, delegation, auction, subscription, or milestone".to_string(),
            "Include party names like: buyer/seller, client/contractor, or provider/consumer".to_string(),
        ]
    } else if confidence < 0.5 {
        vec![
            format!("Detected '{}' pattern with low confidence ({:.0}%).", best_pattern.name(), confidence * 100.0),
            "Consider adding more specific keywords to improve accuracy.".to_string(),
        ]
    } else {
        vec![]
    };
    
    NLMappingResult {
        pattern: best_pattern,
        parties,
        confidence,
        model,
        suggestions,
    }
}

/// Generate model from pattern and parties
fn generate_model(pattern: &ContractPattern, parties: &[String]) -> Option<Model> {
    match pattern {
        ContractPattern::Escrow => {
            let depositor = parties.get(0).map(|s| s.as_str()).unwrap_or("Depositor");
            let deliverer = parties.get(1).map(|s| s.as_str()).unwrap_or("Deliverer");
            Some(templates::escrow(depositor, deliverer))
        }
        ContractPattern::Handshake => {
            let party_a = parties.get(0).map(|s| s.as_str()).unwrap_or("PartyA");
            let party_b = parties.get(1).map(|s| s.as_str()).unwrap_or("PartyB");
            Some(templates::handshake(party_a, party_b))
        }
        ContractPattern::MutualCooperation => {
            let party_a = parties.get(0).map(|s| s.as_str()).unwrap_or("PartyA");
            let party_b = parties.get(1).map(|s| s.as_str()).unwrap_or("PartyB");
            Some(templates::mutual_cooperation(party_a, party_b))
        }
        ContractPattern::AtomicSwap => {
            let party_a = parties.get(0).map(|s| s.as_str()).unwrap_or("PartyA");
            let party_b = parties.get(1).map(|s| s.as_str()).unwrap_or("PartyB");
            Some(templates::atomic_swap(party_a, party_b))
        }
        ContractPattern::Multisig => {
            let signers: Vec<&str> = if parties.is_empty() {
                vec!["Signer1", "Signer2", "Signer3"]
            } else {
                parties.iter().map(|s| s.as_str()).collect()
            };
            let required = (signers.len() / 2) + 1; // Majority
            Some(templates::multisig(&signers, required))
        }
        ContractPattern::ServiceAgreement => {
            let provider = parties.get(0).map(|s| s.as_str()).unwrap_or("Provider");
            let consumer = parties.get(1).map(|s| s.as_str()).unwrap_or("Consumer");
            Some(templates::service_agreement(provider, consumer))
        }
        ContractPattern::Delegation => {
            let principal = parties.get(0).map(|s| s.as_str()).unwrap_or("Principal");
            let agent = parties.get(1).map(|s| s.as_str()).unwrap_or("Agent");
            Some(templates::delegation(principal, agent))
        }
        ContractPattern::Auction => {
            let seller = parties.get(0).map(|s| s.as_str()).unwrap_or("Seller");
            Some(templates::auction(seller))
        }
        ContractPattern::Subscription => {
            let provider = parties.get(0).map(|s| s.as_str()).unwrap_or("Provider");
            let subscriber = parties.get(1).map(|s| s.as_str()).unwrap_or("Subscriber");
            Some(templates::subscription(provider, subscriber))
        }
        ContractPattern::Milestone => {
            let client = parties.get(0).map(|s| s.as_str()).unwrap_or("Client");
            let contractor = parties.get(1).map(|s| s.as_str()).unwrap_or("Contractor");
            // Default milestones if not specified
            Some(templates::milestone(client, contractor, &["Phase1", "Phase2", "Phase3"]))
        }
        ContractPattern::Unknown => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_escrow_detection() {
        let result = map_nl_to_pattern("I want an escrow contract where buyer deposits funds");
        assert_eq!(result.pattern, ContractPattern::Escrow);
        assert!(result.confidence > 0.3);
        assert!(result.model.is_some());
    }
    
    #[test]
    fn test_handshake_detection() {
        let result = map_nl_to_pattern("Both parties must sign to activate the agreement");
        assert_eq!(result.pattern, ContractPattern::Handshake);
        assert!(result.model.is_some());
    }
    
    #[test]
    fn test_delegation_detection() {
        let result = map_nl_to_pattern("Principal delegates authority to agent to act on their behalf");
        assert_eq!(result.pattern, ContractPattern::Delegation);
        assert!(result.parties.contains(&"Principal".to_string()));
        assert!(result.parties.contains(&"Agent".to_string()));
    }
    
    #[test]
    fn test_auction_detection() {
        let result = map_nl_to_pattern("Seller lists item, bidders can bid, highest bidder wins");
        assert_eq!(result.pattern, ContractPattern::Auction);
        assert!(result.parties.contains(&"Seller".to_string()));
    }
    
    #[test]
    fn test_subscription_detection() {
        let result = map_nl_to_pattern("Monthly subscription that can be renewed or cancelled");
        assert_eq!(result.pattern, ContractPattern::Subscription);
    }
    
    #[test]
    fn test_milestone_detection() {
        let result = map_nl_to_pattern("Project with milestone payments at each phase");
        assert_eq!(result.pattern, ContractPattern::Milestone);
    }
    
    #[test]
    fn test_party_extraction() {
        let result = map_nl_to_pattern("Alice and Bob want to cooperate without defection");
        assert!(result.parties.contains(&"Alice".to_string()));
        assert!(result.parties.contains(&"Bob".to_string()));
    }
    
    #[test]
    fn test_unknown_pattern() {
        let result = map_nl_to_pattern("Something completely unrelated to contracts");
        assert_eq!(result.pattern, ContractPattern::Unknown);
        assert!(result.model.is_none());
        assert!(!result.suggestions.is_empty());
    }
    
    #[test]
    fn test_extended_party_names() {
        let result = map_nl_to_pattern("Customer wants to pay merchant for goods");
        assert!(result.parties.contains(&"Customer".to_string()));
        assert!(result.parties.contains(&"Merchant".to_string()));
    }
    
    #[test]
    fn test_service_pattern() {
        let result = map_nl_to_pattern("Freelance work where contractor delivers and gets payment");
        assert_eq!(result.pattern, ContractPattern::ServiceAgreement);
        assert!(result.parties.contains(&"Contractor".to_string()));
    }
}

use anyhow::Result;
use modal_datastore::NetworkDatastore;
use modal_datastore::models::MinerBlock;
use crate::reqres::Response;

/// Handler for POST /data/miner_block/find_ancestor
/// 
/// Efficiently finds the common ancestor between two chains using an iterative binary search approach.
/// Instead of sending all local hashes, the requester sends strategic block hashes and indices,
/// and the responder indicates which ones match their canonical chain.
/// 
/// This allows the requester to use binary search to find the common ancestor in O(log n) requests
/// instead of sending O(n) hashes.
/// 
/// Request format:
/// {
///   "check_points": [
///     { "index": u64, "hash": "string" },
///     ...
///   ]
/// }
/// 
/// Response format:
/// {
///   "chain_length": u64,  // Remote's canonical chain length
///   "matches": [
///     { "index": u64, "hash": "string", "matches": bool },
///     ...
///   ],
///   "highest_match": u64 or null,  // Highest index that matched
///   "cumulative_difficulty": string  // Remote's total cumulative difficulty
/// }
/// 
/// ## Usage Pattern:
/// 
/// The requester should use the following algorithm:
/// 
/// 1. **Initial Request**: Send checkpoints at exponential intervals from the local chain tip:
///    - Indices: [tip, tip-1, tip-2, tip-4, tip-8, tip-16, ...]
///    - Continue until reaching index 0 (genesis)
/// 
/// 2. **Find Upper Bound**: Find the lowest index where chains diverge (first non-match)
/// 
/// 3. **Binary Search**: Between highest_match and the lowest non-match, do binary search:
///    - Check midpoint
///    - If matches: search higher half
///    - If doesn't match: search lower half
/// 
/// 4. **Convergence**: When the search space is exhausted, highest_match is the common ancestor
/// 
/// ## Example:
/// 
/// ```
/// Local chain:  [0] -> [1] -> [2] -> [3] -> [4] -> [5]
/// Remote chain: [0] -> [1] -> [2] -> [3] -> [4'] -> [5'] -> [6']
/// 
/// Request 1: Check [5, 4, 3, 1, 0]
/// Response 1: matches=[false, false, true, true, true], highest_match=3
/// 
/// // Now we know common ancestor is at index 3
/// // We can request blocks from index 4 onwards from remote
/// ```
pub async fn handler(data: Option<serde_json::Value>, datastore: &NetworkDatastore) -> Result<Response> {
    let data = data.unwrap_or_default();
    
    // Parse check_points array
    let check_points = match data.get("check_points").and_then(|v| v.as_array()) {
        Some(arr) => {
            let mut points = Vec::new();
            for item in arr {
                if let (Some(index), Some(hash)) = (
                    item.get("index").and_then(|v| v.as_u64()),
                    item.get("hash").and_then(|v| v.as_str())
                ) {
                    points.push((index, hash.to_string()));
                } else {
                    return Ok(Response {
                        ok: false,
                        data: None,
                        errors: Some(serde_json::json!({
                            "error": "Invalid check_point format. Each must have 'index' (u64) and 'hash' (string)"
                        })),
                    });
                }
            }
            points
        }
        None => {
            return Ok(Response {
                ok: false,
                data: None,
                errors: Some(serde_json::json!({
                    "error": "Missing 'check_points' parameter. Must be array of {index, hash} objects"
                })),
            });
        }
    };
    
    // Load all canonical blocks from datastore
    let canonical_blocks = match MinerBlock::find_all_canonical(datastore).await {
        Ok(blocks) => blocks,
        Err(e) => {
            return Ok(Response {
                ok: false,
                data: None,
                errors: Some(serde_json::json!({"error": format!("Failed to load canonical blocks: {}", e)})),
            });
        }
    };
    
    let chain_length = canonical_blocks.len() as u64;
    
    // Calculate cumulative difficulty
    let cumulative_difficulty = match MinerBlock::calculate_cumulative_difficulty(&canonical_blocks) {
        Ok(diff) => diff.to_string(),
        Err(e) => {
            return Ok(Response {
                ok: false,
                data: None,
                errors: Some(serde_json::json!({
                    "error": format!("Failed to calculate cumulative difficulty: {}", e)
                })),
            });
        }
    };
    
    // Create a hashmap for fast lookups: index -> hash
    let mut index_to_hash = std::collections::HashMap::new();
    for block in &canonical_blocks {
        index_to_hash.insert(block.index, block.hash.clone());
    }
    
    // Check each checkpoint
    let mut matches = Vec::new();
    let mut highest_match: Option<u64> = None;
    
    for (index, hash) in check_points {
        let matches_local = match index_to_hash.get(&index) {
            Some(local_hash) => {
                let is_match = local_hash == &hash;
                if is_match && (highest_match.is_none() || highest_match.unwrap() < index) {
                    highest_match = Some(index);
                }
                is_match
            }
            None => false,  // We don't have a block at this index
        };
        
        matches.push(serde_json::json!({
            "index": index,
            "hash": hash,
            "matches": matches_local,
        }));
    }
    
    Ok(Response {
        ok: true,
        data: Some(serde_json::json!({
            "chain_length": chain_length,
            "matches": matches,
            "highest_match": highest_match,
            "cumulative_difficulty": cumulative_difficulty,
        })),
        errors: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use modal_datastore::Model;
    
    #[tokio::test]
    async fn test_find_ancestor_basic() {
        let datastore = NetworkDatastore::create_in_memory().unwrap();
        
        // Create a chain: [0] -> [1] -> [2] -> [3]
        for i in 0..4 {
            let block = MinerBlock::new_canonical(
                format!("hash_{}", i),
                i,
                0,
                1234567890 + i as i64,
                if i == 0 { "genesis".to_string() } else { format!("hash_{}", i - 1) },
                format!("data_{}", i),
                12345,
                1000,
                "peer_id".to_string(),
                1,
            );
            block.save(&datastore).await.unwrap();
        }
        
        // Test 1: All checkpoints match
        let request = serde_json::json!({
            "check_points": [
                {"index": 3, "hash": "hash_3"},
                {"index": 2, "hash": "hash_2"},
                {"index": 0, "hash": "hash_0"},
            ]
        });
        
        let response = handler(Some(request), &datastore).await.unwrap();
        assert!(response.ok);
        
        let data = response.data.unwrap();
        assert_eq!(data["chain_length"], 4);
        assert_eq!(data["highest_match"], 3);
        
        let matches = data["matches"].as_array().unwrap();
        assert_eq!(matches.len(), 3);
        assert_eq!(matches[0]["matches"], true);
        assert_eq!(matches[1]["matches"], true);
        assert_eq!(matches[2]["matches"], true);
    }
    
    #[tokio::test]
    async fn test_find_ancestor_divergent() {
        let datastore = NetworkDatastore::create_in_memory().unwrap();
        
        // Create a chain: [0] -> [1] -> [2]
        for i in 0..3 {
            let block = MinerBlock::new_canonical(
                format!("hash_{}", i),
                i,
                0,
                1234567890 + i as i64,
                if i == 0 { "genesis".to_string() } else { format!("hash_{}", i - 1) },
                format!("data_{}", i),
                12345,
                1000,
                "peer_id".to_string(),
                1,
            );
            block.save(&datastore).await.unwrap();
        }
        
        // Check points where indices 0,1 match but 2,3 don't (different hashes)
        let request = serde_json::json!({
            "check_points": [
                {"index": 3, "hash": "different_hash_3"},  // Remote has block 3, we don't
                {"index": 2, "hash": "different_hash_2"},  // Diverged at 2
                {"index": 1, "hash": "hash_1"},            // Matches
                {"index": 0, "hash": "hash_0"},            // Matches
            ]
        });
        
        let response = handler(Some(request), &datastore).await.unwrap();
        assert!(response.ok);
        
        let data = response.data.unwrap();
        assert_eq!(data["chain_length"], 3);
        assert_eq!(data["highest_match"], 1);  // Common ancestor at index 1
        
        let matches = data["matches"].as_array().unwrap();
        assert_eq!(matches[0]["matches"], false);  // index 3 - no match
        assert_eq!(matches[1]["matches"], false);  // index 2 - hash mismatch
        assert_eq!(matches[2]["matches"], true);   // index 1 - match
        assert_eq!(matches[3]["matches"], true);   // index 0 - match
    }
    
    #[tokio::test]
    async fn test_find_ancestor_empty_chain() {
        let datastore = NetworkDatastore::create_in_memory().unwrap();
        
        let request = serde_json::json!({
            "check_points": [
                {"index": 0, "hash": "hash_0"},
            ]
        });
        
        let response = handler(Some(request), &datastore).await.unwrap();
        assert!(response.ok);
        
        let data = response.data.unwrap();
        assert_eq!(data["chain_length"], 0);
        assert_eq!(data["highest_match"], serde_json::Value::Null);
    }
    
    #[tokio::test]
    async fn test_find_ancestor_no_common() {
        let datastore = NetworkDatastore::create_in_memory().unwrap();
        
        // Create chain with different hashes
        for i in 0..3 {
            let block = MinerBlock::new_canonical(
                format!("remote_hash_{}", i),
                i,
                0,
                1234567890 + i as i64,
                if i == 0 { "genesis".to_string() } else { format!("remote_hash_{}", i - 1) },
                format!("data_{}", i),
                12345,
                1000,
                "peer_id".to_string(),
                1,
            );
            block.save(&datastore).await.unwrap();
        }
        
        // Check with completely different hashes
        let request = serde_json::json!({
            "check_points": [
                {"index": 2, "hash": "local_hash_2"},
                {"index": 1, "hash": "local_hash_1"},
                {"index": 0, "hash": "local_hash_0"},
            ]
        });
        
        let response = handler(Some(request), &datastore).await.unwrap();
        assert!(response.ok);
        
        let data = response.data.unwrap();
        assert_eq!(data["highest_match"], serde_json::Value::Null);
        
        let matches = data["matches"].as_array().unwrap();
        assert_eq!(matches[0]["matches"], false);
        assert_eq!(matches[1]["matches"], false);
        assert_eq!(matches[2]["matches"], false);
    }
}


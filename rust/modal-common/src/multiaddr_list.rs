use std::str::FromStr;
use anyhow::{Result, Error};
use regex::Regex;
use hickory_resolver::{
    config::{ResolverConfig, ResolverOpts},
    TokioAsyncResolver,
};
use reqwest;
use serde_json::Value;
use libp2p::multiaddr::Multiaddr;

async fn remove_quotes(s: &str) -> String {
    let re = Regex::new(r#""(.+)""#).unwrap();
    if let Some(caps) = re.captures(s) {
        caps[1].to_string()
    } else {
        s.to_string()
    }
}

fn matches_peer_id_suffix(s: &str, peer_id: &str) -> bool {
    let re = Regex::new(&format!(r"/p2p/{}$", peer_id)).unwrap();
    re.is_match(s)
}

#[allow(dead_code)]
async fn resolve_via_cloudflare_dns(name: &str, type_: &str) -> Result<Vec<String>, Error> {
    let client = reqwest::Client::new();
    let url = format!(
        "https://cloudflare-dns.com/dns-query?name={}&type={}",
        name, type_
    );
    
    let response = client
        .get(&url)
        .header("accept", "application/dns-json")
        .send()
        .await?
        .json::<Value>()
        .await?;

    let answers = response["Answer"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|ans| ans["data"].as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    Ok(answers)
}

async fn resolve_via_dns(name: &str, type_: &str) -> Result<Vec<String>, Error> {
    let resolver = TokioAsyncResolver::tokio(ResolverConfig::default(), ResolverOpts::default());

    match type_ {
        "A" => {
            let response = resolver.lookup_ip(name).await?;
            Ok(response
                .iter()
                .map(|ip| ip.to_string())
                .collect())
        }
        "TXT" => {
            let response = resolver.txt_lookup(name).await?;
            let mut results = Vec::new();
            for record in response.iter() {
                let txt = record
                    .txt_data()
                    .iter()
                    .map(|bytes| String::from_utf8_lossy(bytes).into_owned())
                    .collect::<Vec<_>>()
                    .join("");
                results.push(txt);
            }
            Ok(results)
        }
        _ => Ok(vec![]),
    }
}

pub async fn resolve_dns_entries(entries: Vec<String>) -> Result<Vec<String>, Error> {
    let mut results = Vec::new();

    for entry in entries {
        let p2p_re = Regex::new(r"/p2p/(.+)$").unwrap();
        let p2p_match = p2p_re.captures(&entry);

        if entry.starts_with("/dns/") {
            let dns_re = Regex::new(r"^/dns/([A-Za-z0-9-.]+)(.*)").unwrap();
            if let Some(caps) = dns_re.captures(&entry) {
                let name = &caps[1];
                let rest = &caps[2];
                let answers = resolve_via_dns(name, "A").await?;
                let peer_id = p2p_match.as_ref().map(|m| m[1].to_string());

                for address in answers {
                    let ans = format!("/ip4/{}{}", address, rest);
                    if peer_id.is_none() || matches_peer_id_suffix(&ans, &peer_id.as_ref().unwrap()) {
                        results.push(ans);
                    }
                }
            }
        } else if entry.starts_with("/dnsaddr/") {
            let dnsaddr_re = Regex::new(r"^/dnsaddr/([A-Za-z0-9-.]+)(.*)").unwrap();
            if let Some(caps) = dnsaddr_re.captures(&entry) {
                let name = format!("_dnsaddr.{}", &caps[1]);
                let mut answers = resolve_via_dns(&name, "TXT").await?;
                let peer_id = p2p_match.as_ref().map(|m| m[1].to_string());
                
                for answer in &mut answers {
                    *answer = remove_quotes(answer).await;
                    if let Some(ans_caps) = Regex::new(r"^dnsaddr=(.*)").unwrap().captures(answer) {
                        let ans = &ans_caps[1];
                        if peer_id.is_none() || matches_peer_id_suffix(ans, &peer_id.as_ref().unwrap()) {
                            results.push(ans.to_string());
                        }
                    }
                }
            }
        } else {
            results.push(entry);
        }
    }

    Ok(results)
}

pub async fn resolve_dns_multiaddrs(multiaddrs: Vec<Multiaddr>) -> Result<Vec<Multiaddr>, Error> {
    let entries: Vec<String> = multiaddrs.iter().map(|addr| addr.to_string()).collect();
    let resolved_entries = resolve_dns_entries(entries).await?;
    let resolved_multiaddrs = resolved_entries.into_iter().filter_map(|entry| Multiaddr::from_str(&entry).ok()).collect();
    Ok(resolved_multiaddrs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    #[tokio::test]
    async fn test_dns_resolution() { 
        let entries = vec![
            "/dns/example.com/tcp/80/ws/p2p/12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd".to_string()
        ];
        
        let result = resolve_dns_entries(entries).await.unwrap();
        assert!(result[0].starts_with("/ip4/"));
        
        let entries = vec!["/dnsaddr/devnet3.modality.network".to_string()];
        let result = resolve_dns_entries(entries).await.unwrap();
        assert_eq!(result.len(), 3);
        assert!(result[0].starts_with("/ip4/"));
        
        let entries = vec![
            "/dnsaddr/devnet3.modality.network/p2p/12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd".to_string()
        ];
        let result = resolve_dns_entries(entries).await.unwrap();
        assert_eq!(result.len(), 1);
        assert!(result[0].ends_with("/p2p/12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd"));
    }
}
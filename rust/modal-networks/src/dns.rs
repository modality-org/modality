use anyhow::{Context, Result};
use aws_config::BehaviorVersion;
use aws_sdk_route53::{
    types::{Change, ChangeAction, ChangeBatch, ResourceRecord, ResourceRecordSet, RrType},
    Client,
};

use crate::NetworkInfo;

const HOSTED_ZONE: &str = "Z05376073QDH3S1XSX7X7";
const BASE_DOMAIN: &str = "modality.network";

/// DNS manager for updating Route53 records
pub struct DnsManager {
    client: Client,
    hosted_zone_id: String,
    base_domain: String,
}

impl DnsManager {
    /// Create a new DNS manager with default AWS configuration
    pub async fn new() -> Result<Self> {
        let config = aws_config::load_defaults(BehaviorVersion::latest()).await;
        let client = Client::new(&config);

        Ok(Self {
            client,
            hosted_zone_id: HOSTED_ZONE.to_string(),
            base_domain: BASE_DOMAIN.to_string(),
        })
    }

    /// Create a DNS manager with custom configuration
    pub fn with_config(
        client: Client,
        hosted_zone_id: String,
        base_domain: String,
    ) -> Self {
        Self {
            client,
            hosted_zone_id,
            base_domain,
        }
    }

    /// Set TXT records for a network's bootstrappers
    /// Following the dnsaddr protocol: https://github.com/multiformats/multiaddr/blob/master/protocols/DNSADDR.md
    pub async fn set_network_records(&self, network: &NetworkInfo) -> Result<()> {
        let record_name = format!("_dnsaddr.{}.{}", network.name, self.base_domain);
        
        // Convert bootstrapper addresses to dnsaddr TXT records
        let txt_values: Vec<String> = network
            .bootstrappers
            .iter()
            .map(|addr| format!("dnsaddr={}", addr))
            .collect();

        if txt_values.is_empty() {
            println!("No bootstrappers for network {}, skipping DNS update", network.name);
            return Ok(());
        }

        self.set_txt_records(&record_name, &txt_values, 300).await?;
        
        println!("Successfully set DNS records for {}", network.name);
        Ok(())
    }

    /// Set TXT records with multiple values
    async fn set_txt_records(
        &self,
        record_name: &str,
        txt_values: &[String],
        ttl: i64,
    ) -> Result<()> {
        let resource_records: Vec<ResourceRecord> = txt_values
            .iter()
            .map(|value| {
                ResourceRecord::builder()
                    .value(format!("\"{}\"", value))
                    .build()
            })
            .collect::<Result<Vec<_>, _>>()
            .context("Failed to build resource records")?;

        let change = Change::builder()
            .action(ChangeAction::Upsert)
            .resource_record_set(
                ResourceRecordSet::builder()
                    .name(record_name)
                    .r#type(RrType::Txt)
                    .ttl(ttl)
                    .set_resource_records(Some(resource_records))
                    .build()
                    .context("Failed to build resource record set")?,
            )
            .build()
            .context("Failed to build change")?;

        let change_batch = ChangeBatch::builder()
            .set_changes(Some(vec![change]))
            .build()
            .context("Failed to build change batch")?;

        self.client
            .change_resource_record_sets()
            .hosted_zone_id(&self.hosted_zone_id)
            .change_batch(change_batch)
            .send()
            .await
            .context("Failed to update DNS records")?;

        Ok(())
    }

    /// Update DNS records for all networks
    pub async fn update_all_networks(&self, networks: &[NetworkInfo]) -> Result<()> {
        for network in networks {
            println!("Setting records for {}...", network.name);
            if let Err(e) = self.set_network_records(network).await {
                eprintln!("Error setting records for {}: {}", network.name, e);
            }
        }
        Ok(())
    }
}


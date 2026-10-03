/// Network configuration constants
/// Testnet bootstrapper addresses
#[allow(dead_code)]
pub const TESTNET_BOOTSTRAPPERS: &[&str] = &[
    "/dns4/node1.testnet.modality.network/tcp/4040/ws/p2p/12D3KooWJs3d1Q4FdJ2nsuN1ALcU2NvVjZ44khXKuFrCaH12SADx",
    "/dns4/node2.testnet.modality.network/tcp/4040/ws/p2p/12D3KooWN7qfmSKaLkHJ6ABgj2nixEjta8DUX7K1S8oth5rwcgSi",
    "/dns4/node3.testnet.modality.network/tcp/4040/ws/p2p/12D3KooWJSVWV2YSqXrEgqytBok672qRs5BS2EseKe8kzhEWXyJi",
];

/// Default autoupgrade base URL
#[allow(dead_code)]
pub const DEFAULT_AUTOUPGRADE_BASE_URL: &str = "https://get.modality.org";

/// Default autoupgrade check interval in seconds
#[allow(dead_code)]
pub const DEFAULT_AUTOUPGRADE_CHECK_INTERVAL_SECS: u64 = 3600;

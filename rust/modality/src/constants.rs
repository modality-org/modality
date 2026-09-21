/// Network configuration constants
/// Testnet bootstrapper addresses
#[allow(dead_code)]
pub const TESTNET_BOOTSTRAPPERS: &[&str] = &[
    "/dns4/node1.testnet.modality.network/tcp/4040/ws/p2p/12D3KooWE4NPREQxLkevA5Rxd61Xiue4tTkUGN22qNABD7Mw5JhM",
    "/dns4/node2.testnet.modality.network/tcp/4040/ws/p2p/12D3KooWJpFYTRHNuPfwoj1hTf87aqB7CDJHKtVFp3RhPNB1DrRw",
    "/dns4/node3.testnet.modality.network/tcp/4040/ws/p2p/12D3KooWLHTsoeBE1ZWBgzumeSi6hsm3o9AndFufrGx7xLTyq2dw",
];

/// Default autoupgrade base URL
#[allow(dead_code)]
pub const DEFAULT_AUTOUPGRADE_BASE_URL: &str = "https://get.modality.org";

/// Default autoupgrade check interval in seconds
#[allow(dead_code)]
pub const DEFAULT_AUTOUPGRADE_CHECK_INTERVAL_SECS: u64 = 3600;

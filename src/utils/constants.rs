use std::path::PathBuf;
use crate::utils::secure_fs;

pub fn wallet_file_path() -> PathBuf {
    let dir = dirs::data_local_dir()
        .expect("Failed to get data directory")
        .join("rsk-rust-cli");

    // Ensure the directory exists with secure permissions
    secure_fs::create_dir_secure(&dir).expect("Failed to create wallet directory");

    dir.join("rsk-rust-cli.json")
}

pub const METHOD_TYPES: &str = "read";

pub const ALLOWED_BRIDGE_METHODS: &[(&str, &[&str])] = &[
    (
        "read",
        &[
            "getBtcBlockchainBestChainHeight",
            "getStateForBtcReleaseClient",
            "getStateForDebugging",
            "getBtcBlockchainInitialBlockHeight",
            "getBtcBlockchainBlockHashAtDepth",
            "getBtcTxHashProcessedHeight",
            "isBtcTxHashAlreadyProcessed",
            "getFederationAddress",
            "getFederationSize",
            "getFederationThreshold",
            "getFederatorPublicKey",
            "getFederatorPublicKeyOfType",
            "getFederationCreationTime",
            "getFederationCreationBlockNumber",
            "getRetiringFederationAddress",
            "getRetiringFederationSize",
            "getRetiringFederationThreshold",
            "getRetiringFederatorPublicKeyOfType",
            "getRetiringFederationCreationTime",
            "getRetiringFederationCreationBlockNumber",
            "getPendingFederationHash",
            "getPendingFederationSize",
            "getPendingFederatorPublicKeyOfType",
            "getFeePerKb",
            "getMinimumLockTxValue",
            "getBtcTransactionConfirmations",
            "getLockingCap",
            "hasBtcBlockCoinbaseTransactionInformation",
            "getActiveFederationCreationBlockHeight",
            "getBtcBlockchainBestBlockHeader",
            "getBtcBlockchainBlockHeaderByHash",
            "getBtcBlockchainBlockHeaderByHeight",
            "getBtcBlockchainParentBlockHeaderByHash",
            "getEstimatedFeesForNextPegOutEvent",
            "getNextPegoutCreationBlockNumber",
            "getQueuedPegoutsCount",
            "getActivePowpegRedeemScript",
        ],
    ),
    (
        "write",
        &[
            "registerBtcTransaction",
            "registerBtcCoinbaseTransaction",
            "receiveHeader",
        ],
    ),
];

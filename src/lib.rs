
pub mod blockchain;
pub mod token;
pub mod smart_contract;
pub mod transaction;
pub mod quantum_crypto;
pub mod database;
pub mod key_manager;
pub mod app;


// Re-exports principais
pub use app::QuantumBlockchainApp;
pub use blockchain::Blockchain;
pub use token::Token;
pub use smart_contract::SmartContract;
pub use transaction::{Transaction, SecureTransaction, TransactionError};
pub use quantum_crypto::QuantumCrypto;
pub use database::Database;
pub use key_manager::KeyManager;
pub use quantum_crypto::quantum_crypto::OqsError;

// Constantes globais
pub const BLOCKCHAIN_FILE: &str = "blockchain.json";
pub const DB_PATH: &str = "blockchain.db";
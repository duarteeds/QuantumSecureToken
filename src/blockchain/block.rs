use serde::{Serialize, Deserialize};
use std::time::{SystemTime, UNIX_EPOCH};
use crate::quantum_crypto::{QuantumCrypto, OqsError};
use crate::transaction::SecureTransaction;
use crate::smart_contract::SmartContract;  // Importe do módulo correto
use std::vec::Vec; 

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub index: u64,
    pub timestamp: u64,
    pub transactions: Vec<SecureTransaction>,
    pub contracts: Vec<SmartContract>,
    pub previous_hash: String,
    pub hash: String,
    pub validator_signature: Option<Vec<u8>>,
}

impl Block {
    pub fn new(
        index: u64,
        transactions: Vec<SecureTransaction>,
        contracts: Vec<SmartContract>,
        previous_hash: String,
    ) -> Result<Self, OqsError> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let hash = Self::calculate_hash(
            index,
            timestamp,
            &transactions,
            &contracts,
            &previous_hash,
        )?;

        Ok(Block {
            index,
            timestamp,
            transactions,
            contracts,
            previous_hash,
            hash,
            validator_signature: None,
        })
    }

    /// Calcula o hash do bloco.
    pub fn calculate_hash(
    index: u64,
    timestamp: u64,
    transactions: &Vec<SecureTransaction>,
    contracts: &Vec<SmartContract>,
    previous_hash: &str,
) -> Result<String, OqsError> {
    // Serializa os dados de forma determinística
    let data = format!("{}{}{:?}{:?}{}", 
        index,
        timestamp,
        transactions,
        contracts,
        previous_hash
    );

    let crypto = QuantumCrypto::new()
        .map_err(OqsError::from)?;

    let (encrypted_data, _, _) = crypto.encrypt(data.as_bytes())
        .map_err(OqsError::from)?;

    // Hash resistente a ataques quânticos usando Dilithium5
    Ok(hex::encode(encrypted_data))
}
}
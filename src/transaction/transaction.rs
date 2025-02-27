﻿#![allow(dead_code)]
//Verificado com Segurança maxima DS

use oqs::Error as OqsError;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::error::Error as StdError;
use super::secure_transaction::SecureTransaction;
use pqcrypto_dilithium::dilithium5::{SecretKey, PublicKey, sign, DetachedSignature,verify_detached_signature, keypair};
use std::collections::HashMap;
use bincode::{serialize, deserialize};
use pqcrypto_traits::sign::PublicKey as PqcPublicKey;
use pqcrypto_traits::sign::SignedMessage;
use pqcrypto_traits::sign::DetachedSignature as PqcDetachedSignature;
use regex::Regex;
use once_cell::sync::Lazy;
use zeroize::Zeroize;
use tracing::{info, warn};
use std::sync::Mutex;
use pqcrypto_dilithium::dilithium5::detached_sign;


// Constantes de segurança
const MAX_TRANSACTION_SIZE: usize = 128 * 1024;  // 128KB
const MAX_SIGNATURE_SIZE: usize = 4627;     // 64KB
const TIMESTAMP_WINDOW: i64 = 300;              // 5 minutes
const MIN_ADDRESS_LENGTH: usize = 32;
const MAX_ADDRESS_LENGTH: usize = 64;
static HASH_SALT: &[u8] = b"QUANTUM_SECURE_TRANSACTION_V1";
const MAX_AMOUNT: u64 = 1_000_000_000;
const MIN_AMOUNT: u64 = 1;

// Enum de erros com tratamento completo
#[derive(Debug)]
pub enum TransactionError {
    OqsError(OqsError),
    InvalidTransaction,
    InvalidDataFormat,
    InvalItIdentifierFunds,
    NoneCipherflow,
    TimestampInvalid,
    AddressFormatInvalid,
    SignaturesizeExceeded,
    DataSizeExceeded,
    CryptoError(String),
    InvalidSignatures(String),
    InvalidBuildKey(String),
    InvalidData(String),
    NonceOverflow, 
    SignatureSizeExceeded, 
}


impl From<OqsError> for TransactionError {
    fn from(err: OqsError) -> Self {
        TransactionError::OqsError(err) 
    }
}

impl fmt::Display for TransactionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransactionError::OqsError(err) => write!(f, "OutError: {}", err),
            TransactionError::InvalidTransaction => write!(f, "Invalid transaction"),
            TransactionError::InvalidDataFormat => write!(f, "Invalid data format"),
            TransactionError::InvalItIdentifierFunds => write!(f, "Invalid identifier funds"),
            TransactionError::NoneCipherflow => write!(f, "No cipherflow"),
            TransactionError::TimestampInvalid => write!(f, "Invalid timestamp"),
            TransactionError::AddressFormatInvalid => write!(f, "Invalid address format"),
            TransactionError::SignaturesizeExceeded => write!(f, "Signature size exceeded"),
            TransactionError::DataSizeExceeded => write!(f, "Data size exceeded"),
            TransactionError::CryptoError(msg) => write!(f, "Crypto error: {}", msg),
            TransactionError::InvalidSignatures(msg) => write!(f, "Invalid signatures: {}", msg),
            TransactionError::InvalidBuildKey(msg) => write!(f, "Invalid build key: {}", msg),
            TransactionError::InvalidData(msg) => write!(f, "Invalid data: {}", msg),
            TransactionError::NonceOverflow => write!(f, "Nonce overflow"),
            TransactionError::SignatureSizeExceeded => write!(f, "Signature size exceeded"),

        }
    }
}

impl StdError for TransactionError {}

pub struct NonceRegistry {
    registry: Mutex<HashMap<String, u64>>,
    max_nonce_gap: u64,
    last_update: Mutex<HashMap<String, i64>>,
    min_update_interval: i64,
}

impl NonceRegistry {
    pub fn new() -> Self {
        NonceRegistry {
            registry: Mutex::new(HashMap::new()),
            max_nonce_gap: 1000,
            last_update: Mutex::new(HashMap::new()),
            min_update_interval: 1,
        }
    }

    pub fn validate_nonce(&mut self, address: &str, nonce: u64) -> Result<(), TransactionError> {
    let registry = self.registry.get_mut().unwrap();
    let last_nonce = registry.get(address).copied().unwrap_or(0);
    
    if nonce <= last_nonce {
        return Err(TransactionError::NonceOverflow);
    }
    
    registry.insert(address.to_string(), nonce);
    Ok(())
}

}

#[derive(Debug, Serialize, Deserialize, Zeroize)]
pub struct Transaction {
    pub token_id: u64,
    pub from: String,
    pub to: String,
    pub amount: u64,
    pub timestamp: i64,
    pub nonce: u64,
    pub public_key: Vec<u8>,
    pub signature: Vec<u8>,
    pub transaction_hash: Vec<u8>,
}


impl From<SecureTransaction> for Transaction {
    fn from(st: SecureTransaction) -> Self {
        let mut transaction = Transaction {
            token_id: 0,
            from: st.from.clone(),
            to: st.to.clone(),
            amount: st.amount,
            timestamp: st.timestamp,
            nonce: st.nonce,
            public_key: Vec::new(),
            signature: Vec::new(),
            transaction_hash: Vec::new(),
        };
        let _ = transaction.update_hash();
        transaction
    }
}

impl Transaction {
    pub fn new(from: String, to: String, amount: u64, public_key: Vec<u8>) -> Result<Self, TransactionError> {
        // Validate addresses first
        if from.len() < MIN_ADDRESS_LENGTH || from.len() > MAX_ADDRESS_LENGTH ||
           to.len() < MIN_ADDRESS_LENGTH || to.len() > MAX_ADDRESS_LENGTH {
            return Err(TransactionError::AddressFormatInvalid);
        }

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| TransactionError::TimestampInvalid)?
            .as_secs() as i64;

        let mut transaction = Transaction {
            token_id: 0,
            from,
            to,
            amount,
            timestamp,
            nonce: 1,
            public_key,
            signature: Vec::new(),
            transaction_hash: Vec::new(),
        };

        transaction.update_hash()?;
        Ok(transaction)
    }

     fn serialize_for_signing(&self) -> Result<Vec<u8>, TransactionError> {
    // Estrutura exata para assinatura
    #[derive(Serialize)]
    struct SignableData {
        token_id: u64,
        from: String,
        to: String,
        amount: u64,
        timestamp: i64,
        nonce: u64,
        public_key: Vec<u8>
    }

    let data = SignableData {
        token_id: self.token_id,
        from: self.from.clone(),
        to: self.to.clone(),
        amount: self.amount,
        timestamp: self.timestamp,
        nonce: self.nonce,
        public_key: self.public_key.clone()
    };

    bincode::serialize(&data)
        .map_err(|_| TransactionError::InvalidDataFormat)
}


    fn calculate_hash(&self) -> Result<Vec<u8>, TransactionError> {
    #[derive(Serialize)]
    struct TransactionHashData<'a> {
        token_id: u64,
        from: &'a str,
        to: &'a str,
        amount: u64,
        timestamp: i64,
        nonce: u64,
        public_key: &'a [u8],
        salt: &'a [u8],
        signature: &'a [u8]  // Added signature to hash calculation
    }

    let hash_data = TransactionHashData {
        token_id: self.token_id,
        from: &self.from,
        to: &self.to,
        amount: self.amount,
        timestamp: self.timestamp,
        nonce: self.nonce,
        public_key: &self.public_key,
        salt: HASH_SALT,
        signature: &self.signature
    };

    let data = serialize(&hash_data)
        .map_err(|_| TransactionError::InvalidDataFormat)?;

    static HASH_KEYS: Lazy<(PublicKey, SecretKey)> = Lazy::new(|| keypair());
    let signature = sign(&data, &HASH_KEYS.1);
    Ok(signature.as_bytes().to_vec())
}

    pub fn update_hash(&mut self) -> Result<(), TransactionError> {
        self.transaction_hash = self.calculate_hash()?;
        Ok(())
    }

    pub fn sign(&mut self, secret_key: &SecretKey) -> Result<(), TransactionError> {
    self.validate_address()?;
    let data = self.serialize_for_signing()?;
    
    // Primeiro cria a SignedMessage
   
    let signature = detached_sign(&data, secret_key);
    self.signature = signature.as_bytes().to_vec();
    
    // Verify signature size before proceeding
    if self.signature.len() > MAX_SIGNATURE_SIZE {
        return Err(TransactionError::SignatureSizeExceeded);
    }
    
    self.update_hash()?;
    Ok(())
}

    pub fn verify(&self, public_key: &PublicKey) -> Result<(), TransactionError> {
        let data = self.serialize_for_signing()?;
        
        // Validate signature size before attempting conversion
        if self.signature.len() > MAX_SIGNATURE_SIZE {
            return Err(TransactionError::SignatureSizeExceeded);
        }
        
        let signature = DetachedSignature::from_bytes(&self.signature)
            .map_err(|_| TransactionError::InvalidSignatures("Invalid signature".to_string()))?;
            
        verify_detached_signature(&signature, &data, public_key)
            .map_err(|_| TransactionError::InvalidSignatures("Invalid signature".to_string()))?;
            
        Ok(())
    }

    // Add helper method to get the correct signature size
    pub fn get_expected_signature_size() -> usize {
        4627 // Expected size for Dilithium5 signatures
    }

    pub fn validate_address(&self) -> Result<(), TransactionError> {
        let re = Regex::new(&format!(
            "^[a-zA-Z0-9]{{{},{}}}$",
            MIN_ADDRESS_LENGTH,
            MAX_ADDRESS_LENGTH
        )).map_err(|_| TransactionError::AddressFormatInvalid)?;

        if !re.is_match(&self.from) || !re.is_match(&self.to) {
            warn!("Endereço inválido detectado. From: {}, To: {}", self.from, self.to);
            return Err(TransactionError::AddressFormatInvalid);
        }

        if self.from == self.to {
            warn!("Tentativa de transação para o mesmo endereço: {}", self.from);
            return Err(TransactionError::AddressFormatInvalid);
        }

        Ok(())
    }

    fn validate_timestamp(&self) -> Result<(), TransactionError> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| TransactionError::TimestampInvalid)?
            .as_secs() as i64;

        match now.checked_sub(self.timestamp) {
            Some(diff) if diff.abs() <= TIMESTAMP_WINDOW => Ok(()),
            _ => {
                warn!("Timestamp inválido detectado. From: {}", self.from);
                Err(TransactionError::TimestampInvalid)
            }
        }
    }

    pub fn validate(&self, nonce_registry: &mut NonceRegistry) -> Result<(), TransactionError> {
    if self.amount < MIN_AMOUNT || self.amount > MAX_AMOUNT {
        return Err(TransactionError::InvalidData("Valor de transação inválido".to_string()));
    }

    self.validate_address()?;
    self.validate_timestamp()?;
    
    // Validate nonce first, before other expensive operations
    nonce_registry.validate_nonce(&self.from, self.nonce)?;

    // Verify transaction size
    if serialize(self).map_err(|_| TransactionError::InvalidDataFormat)?.len() > MAX_TRANSACTION_SIZE {
        return Err(TransactionError::DataSizeExceeded);
    }

    Ok(())
}

    fn deserialize_from_bytes(data: &[u8]) -> Result<Self, TransactionError> {
        if data.len() > MAX_TRANSACTION_SIZE {
            warn!("Tamanho de dados excedido na desserialização: {}", data.len());
            return Err(TransactionError::DataSizeExceeded);
        }
        deserialize(data).map_err(|_| TransactionError::InvalidDataFormat)
    }
}

pub fn generate_keys() -> Result<(SecretKey, PublicKey), TransactionError> {
    let (public_key, secret_key) = keypair();
    info!("Novo par de chaves gerado");
    Ok((secret_key, public_key))
}

pub fn example() -> Result<(), TransactionError> {
    let (secret_key, public_key) = generate_keys()?;

    let mut transaction = Transaction::new(
        "remetente".to_string(),
        "destinatário".to_string(),
        100,
        public_key.as_bytes().to_vec(),
    )?;

    transaction.sign(&secret_key)?;  // Adicionado ponto e vírgula
    transaction.verify(&public_key)?;

    Ok(())
}

impl Clone for Transaction {
    fn clone(&self) -> Self {
        Transaction {
            token_id: self.token_id,
            from: self.from.clone(),
            to: self.to.clone(),
            amount: self.amount,
            timestamp: self.timestamp,
            nonce: self.nonce,
            public_key: self.public_key.clone(),
            signature: self.signature.clone(),
            transaction_hash: self.transaction_hash.clone(),
        }
    }
}

impl Drop for Transaction {
    fn drop(&mut self) {
        self.public_key.fill(0);
        self.signature.fill(0);
        self.transaction_hash.fill(0);
    }
}


 #[cfg(test)]
mod tests {
    use super::*;

    fn create_valid_transaction() -> Result<(Transaction, SecretKey, PublicKey), TransactionError> {
        let (secret_key, public_key) = generate_keys()?;
        
        let mut transaction = Transaction::new(
            "a".repeat(MIN_ADDRESS_LENGTH),
            "b".repeat(MIN_ADDRESS_LENGTH),
            100,
            public_key.as_bytes().to_vec(),
        )?;
        
        transaction.sign(&secret_key)?;
        
        Ok((transaction, secret_key, public_key))
    }

    #[test]
    fn test_transaction_validation() -> Result<(), TransactionError> {
        let (transaction, _, public_key) = create_valid_transaction()?;
        let mut nonce_registry = NonceRegistry::new();
        
        transaction.validate(&mut nonce_registry)?;
        transaction.verify(&public_key)?;
        
        Ok(())
    }

    #[test]
    fn test_invalid_amount() -> Result<(), TransactionError> {
        let (mut transaction, secret_key, _) = create_valid_transaction()?;
        let mut nonce_registry = NonceRegistry::new();

        transaction.amount = MAX_AMOUNT + 1;
        transaction.sign(&secret_key)?;

        assert!(matches!(
            transaction.validate(&mut nonce_registry),
            Err(TransactionError::InvalidData(_))
        ));
        
        Ok(())
    }

    #[test]
    fn test_replay_attack() -> Result<(), TransactionError> {
        let mut nonce_registry = NonceRegistry::new();
        let (transaction1, _secret_key, _) = create_valid_transaction()?;
        transaction1.validate(&mut nonce_registry)?;

        let transaction2 = transaction1.clone();
        assert!(matches!(
            transaction2.validate(&mut nonce_registry),
            Err(TransactionError::NonceOverflow)
        ));
        Ok(())
    }

    #[test]
    fn test_timestamp_validation() -> Result<(), TransactionError> {
        let (mut transaction, secret_key, _) = create_valid_transaction()?;
        
        transaction.timestamp = 0;
        transaction.sign(&secret_key)?;
        
        assert!(matches!(
            transaction.validate_timestamp(),
            Err(TransactionError::TimestampInvalid)
        ));
        
        Ok(())
    }

    #[test]
    fn test_address_validation() -> Result<(), TransactionError> {
        let result = Transaction::new(
            "short".to_string(),
            "b".repeat(MIN_ADDRESS_LENGTH),
            100,
            vec![0; 32],
        );
        
        assert!(matches!(
            result,
            Err(TransactionError::AddressFormatInvalid)
        ));
        
        Ok(())
    }

    #[test]
    fn test_signature_size() -> Result<(), TransactionError> {
        let (transaction, _, public_key) = create_valid_transaction()?;
        
        assert_eq!(
            transaction.signature.len(),
            Transaction::get_expected_signature_size(),
            "Signature size mismatch"
        );
            
        transaction.verify(&public_key)?;
        
        Ok(())
    }

    #[test]
    fn test_transaction_serialization() -> Result<(), TransactionError> {
        let (transaction, _, _) = create_valid_transaction()?;
        let data = transaction.serialize_for_signing()?;
        assert!(!data.is_empty());
        assert!(data.len() <= MAX_TRANSACTION_SIZE);
        Ok(())
    }

    // Novo teste adicionado
    #[test]
    fn test_signature_generation_and_size() -> Result<(), TransactionError> {
        let (mut transaction, secret_key, public_key) = create_valid_transaction()?;
        
        // Limpa a assinatura existente e assina novamente para garantir
        transaction.signature.clear();
        transaction.sign(&secret_key)?;
        
        assert_eq!(
            transaction.signature.len(),
            Transaction::get_expected_signature_size(),
            "Signature size mismatch"
        );
        
        // Verifica se a assinatura é válida
        transaction.verify(&public_key)?;
        Ok(())
    }
}
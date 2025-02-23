
use oqs::kem::{Algorithm as KemAlgorithm, Kem};
use oqs::sig::{Sig, Algorithm as SigAlgorithm};
use anyhow::{Result, Context};
use rusqlite::{Connection, TransactionBehavior};
use std::time::{SystemTime, UNIX_EPOCH};
use rusqlite::params;

pub struct KeyManager {
    kem: Kem,
    sig: Sig,
}

impl KeyManager {
    pub fn new() -> Result<Self> {
        let kem = Kem::new(KemAlgorithm::Kyber512)
            .context("Falha ao inicializar Kyber512")?;
        let sig = Sig::new(SigAlgorithm::Dilithium2)
            .context("Falha ao inicializar Dilithium2")?;
        
        Ok(Self { kem, sig })
    }

    pub fn generate_quantum_keys(&self) -> Result<(Vec<u8>, Vec<u8>)> {
        let (public_key, secret_key) = self.kem.keypair()
            .context("Falha ao gerar par de chaves Kyber")?;
        
        Ok((public_key.into_vec(), secret_key.into_vec()))
    }

    pub fn generate_signing_keys(&self) -> Result<(Vec<u8>, Vec<u8>)> {
        let (public_key, secret_key) = self.sig.keypair()
            .context("Falha ao gerar par de chaves Dilithium")?;
        
        Ok((public_key.into_vec(), secret_key.into_vec()))
    }

    pub fn create_secure_transaction<'a>(
        &self,
        from: String,
        to: String,
        amount: u64,
        conn: &'a mut Connection
    ) -> Result<rusqlite::Transaction<'a>> {
        let (pub_key, secret_key_bytes) = self.generate_signing_keys()?;
        
        let transaction = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let message = format!(
            "{}:{}:{}:{}",
            from,
            to,
            amount,
            timestamp
        );

        let signature = self.sig.sign(message.as_bytes(), 
            &self.sig.secret_key_from_bytes(&secret_key_bytes)
                .ok_or_else(|| anyhow::anyhow!("Falha ao criar chave secreta"))?
        )?;

        transaction.execute(
            "INSERT INTO transactions (from_address, to_address, amount, timestamp, signature, public_key) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                from,
                to,
                amount,
                timestamp,
                signature.as_ref(),
                pub_key
            ],
        )?;

        Ok(transaction)
    }
}
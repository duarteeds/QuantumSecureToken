use pqcrypto_dilithium::dilithium5::{verify_detached_signature, detached_sign, DetachedSignature, SecretKey, PublicKey};
use pqcrypto_traits::sign::{PublicKey as TraitsPublicKey, DetachedSignature as PqcDetachedSignature};
use sodiumoxide::crypto::secretbox;
use crate::transaction::TransactionError;
use serde::{Serialize, Deserialize};
use secrecy::Zeroize;


const MAX_SIGNATURE_SIZE: usize = 4627; // Tamanho da assinatura Dilithium5

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SecureTransaction {
    pub from: String,
    pub to: String,
    pub amount: u64,
    pub timestamp: i64,
    pub nonce: u64,
    pub signature: Vec<u8>,
    pub public_key: Vec<u8>,
    pub cipher_key: Vec<u8>,
    pub encrypted_data: Vec<u8>,
    pub iv: Vec<u8>,          // Vetor de inicialização para criptografia
    pub salt: Vec<u8>,        // Salt para derivação de chave
    pub mac: Vec<u8>,         // Código de Autenticação de Mensagem (MAC)
}

impl SecureTransaction {
    pub fn new(
        from: String,
        to: String,
        amount: u64,
        timestamp: i64,
        nonce: u64,
    ) -> Result<Self, TransactionError> {
        // Gera um novo par de chaves Dilithium5
        let (public_key, secret_key) = generate_keypair()?;

        // Gera salt e IV aleatórios
        let salt = secretbox::gen_nonce().0.to_vec();
        let iv = secretbox::gen_nonce().0.to_vec();

        // Gera chave de cifra
        let cipher_key = secretbox::gen_key().0.to_vec();

        // Cria a transação inicial
        let mut transaction = SecureTransaction {
            from,
            to,
            amount,
            timestamp,
            nonce,
            signature: Vec::new(),
            public_key: public_key.as_bytes().to_vec(),
            cipher_key,
            encrypted_data: Vec::new(),
            iv,
            salt,
            mac: Vec::new(),
        };

        // Encripta os dados
        transaction.encrypt_data(&secret_key)?;

        // Gera a assinatura
        transaction.sign(&secret_key)?;

        Ok(transaction)
    }

    pub fn verify(&self) -> Result<bool, TransactionError> {
        // Verifica o tamanho da assinatura
        if self.signature.len() > MAX_SIGNATURE_SIZE {
            return Err(TransactionError::InvalidSignature("Signature too large".to_string()));
        }

        // Verifica os dados encriptados
        let data = self.decrypt_data()?;

        // Verifica o MAC
        if !self.verify_mac()? {
            return Err(TransactionError::InvalidSignature("Signature too large".to_string()));
        }

        // Converte a assinatura
        let signature = DetachedSignature::from_bytes(&self.signature)
            .map_err(|_| TransactionError::InvalidSignature("Invalid signature bytes".to_string()))?;

        // Converte a chave pública
        let public_key = PublicKey::from_bytes(&self.public_key)
            .map_err(|_| TransactionError::InvalidPublicKey("Invalid public key".to_string()))?;

        // Verifica a assinatura e os dados da transação
        verify_detached_signature(&signature, &data, &public_key)
            .map_err(|_| TransactionError::InvalidSignature("Invalid signature".to_string()))?;

        // Sucesso: a assinatura é válida
        Ok(true)
    }

    fn encrypt_data(&mut self, _secret_key: &SecretKey) -> Result<(), TransactionError> {
        let data = self.serialize_data()?;

        // Gera nonce para encriptação
        let nonce = secretbox::Nonce::from_slice(&self.iv)
            .ok_or(TransactionError::InvalidData("Invalid nonce".to_string()))?;

        // Gera chave para encriptação
        let key = secretbox::Key::from_slice(&self.cipher_key)
            .ok_or(TransactionError::InvalidData("Invalid key".to_string()))?;

        // Encripta os dados
        self.encrypted_data = secretbox::seal(&data, &nonce, &key);

        // Gera MAC
        self.generate_mac(&data)?;

        Ok(())
    }

    fn decrypt_data(&self) -> Result<Vec<u8>, TransactionError> {
        let nonce = secretbox::Nonce::from_slice(&self.iv)
            .ok_or(TransactionError::InvalidData("Invalid nonce".to_string()))?;

        let key = secretbox::Key::from_slice(&self.cipher_key)
            .ok_or(TransactionError::InvalidData("Invalid key".to_string()))?;

        secretbox::open(&self.encrypted_data, &nonce, &key)
            .map_err(|_| TransactionError::InvalidData("Decryption failed".to_string()))
    }

    fn generate_mac(&mut self, data: &[u8]) -> Result<(), TransactionError> {
        let key = secretbox::Key::from_slice(&self.cipher_key)
            .ok_or(TransactionError::InvalidData("Invalid key".to_string()))?;

        self.mac = secretbox::seal(data, &secretbox::Nonce::from_slice(&self.iv)
            .ok_or(TransactionError::InvalidData("Invalid nonce".to_string()))?, &key);

        Ok(())
    }

    fn verify_mac(&self) -> Result<bool, TransactionError> {
        let key = secretbox::Key::from_slice(&self.cipher_key)
            .ok_or(TransactionError::InvalidData("Invalid key".to_string()))?;

        let nonce = secretbox::Nonce::from_slice(&self.iv)
            .ok_or(TransactionError::InvalidData("Invalid nonce".to_string()))?;

        match secretbox::open(&self.mac, &nonce, &key) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    fn sign(&mut self, secret_key: &SecretKey) -> Result<(), TransactionError> {
        let data = self.serialize_data()?;
        self.signature = detached_sign(&data, secret_key).as_bytes().to_vec();
        Ok(())
    }

    fn serialize_data(&self) -> Result<Vec<u8>, TransactionError> {
        let data = format!("{}:{}:{}:{}:{}",
            self.from,
            self.to,
            self.amount,
            self.timestamp,
            self.nonce
        );

        Ok(data.into_bytes())
    }
}

// Gera um par de chaves Dilithium5
fn generate_keypair() -> Result<(PublicKey, SecretKey), TransactionError> {
    let (pk, sk) = pqcrypto_dilithium::dilithium5::keypair();
    Ok((pk, sk))
}

// Limpeza segura de dados sensíveis
impl Drop for SecureTransaction {
    fn drop(&mut self) {
        self.cipher_key.zeroize();
        self.signature.zeroize();
        self.encrypted_data.zeroize();
        self.mac.zeroize();
    }
}
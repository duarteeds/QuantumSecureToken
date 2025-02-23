use crate::transaction::{Transaction, TransactionError};
use std::time::{SystemTime, UNIX_EPOCH};
use super::blockchain::Blockchain;

#[derive(Debug)]
pub enum TransacaoErro {
    EnderecoInvalido,
    TokenNaoEncontrado,
    SaldoInsuficiente,
    NonceInvalido,
    TimestampInvalido,
    TransacaoRepetida,
    ValorInvalido,
}

pub fn validar_transacao(
    transaction: &Transaction,
    blockchain: &Blockchain,
    nonce_atual: u64,
) -> Result<(), TransacaoErro> {
    // Validar formato dos endereços
    if !validar_formato_endereco(&transaction.from) || !validar_formato_endereco(&transaction.to) {
        return Err(TransacaoErro::EnderecoInvalido);
    }

    // Validar token
    let token = match blockchain.get_token(&transaction.token_id.to_string()) {
        Some(t) => t,
        None => return Err(TransacaoErro::TokenNaoEncontrado),
    };

    // Validar saldo
    if token.balance_of(&transaction.from) < transaction.amount {
        return Err(TransacaoErro::SaldoInsuficiente);
    }

    // Validar nonce
    if transaction.nonce != nonce_atual + 1 {
        return Err(TransacaoErro::NonceInvalido);
    }

    // Validar valor
    if transaction.amount == 0 {
        return Err(TransacaoErro::ValorInvalido);
    }

    // Validar timestamp (não mais antigo que 24 horas)
    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    
    if transaction.timestamp < current_time - 86400 {
        return Err(TransacaoErro::TimestampInvalido);
    }

    // Verificar se a transação é repetida
    if blockchain.transacao_existe(transaction) {
        return Err(TransacaoErro::TransacaoRepetida);
    }

    Ok(())
}

fn validar_formato_endereco(endereco: &str) -> bool {
    // Implementação básica: verifica se é um endereço Ethereum válido
    if !endereco.starts_with("0x") {
        return false;
    }

    if endereco.len() != 42 {
        return false;
    }

    // Verifica se contém apenas caracteres hexadecimais após "0x"
    endereco[2..].chars().all(|c| c.is_ascii_hexdigit())
}

// Função auxiliar para verificar se uma transação já existe
impl Blockchain {
    pub fn transacao_existe(&self, transaction: &Transaction) -> bool {
        self.chain.iter().any(|block| {
            block.transactions.iter().any(|tx| {
                tx.from == transaction.from 
                && tx.nonce == transaction.nonce
                && tx.timestamp == transaction.timestamp
            })
        })
    }

    pub fn validar_timestamp(timestamp: i64) -> Result<(), TransactionError> {
    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    
    if timestamp > current_time || current_time - timestamp > 86400 {
        return Err(TransactionError::TimestampInvalido);
    }
    Ok(())

    }

    pub fn validar_assinatura(transaction: &Transaction, public_key: &[u8], signature: &[u8]) -> bool {
        use pqcrypto_dilithium::dilithium5::verify_detached_signature;
        use pqcrypto_traits::sign::{DetachedSignature, PublicKey};

        let pk = PublicKey::from_bytes(public_key).unwrap();
        let sig = DetachedSignature::from_bytes(signature).unwrap();

        let data = format!(
            "{}:{}:{}:{}",
            transaction.from, transaction.to, transaction.amount, transaction.timestamp
        );
        verify_detached_signature(&sig, data.as_bytes(), &pk).is_ok()
    }
}
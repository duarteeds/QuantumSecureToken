
use std::fmt;
use oqs::Error; 


#[derive(Debug)]
pub enum TransactionError {
    OqsError(Error),
    InvalidTransaction,
    InvalidDataFormat,
    InsufficientFunds,
    NonceInvalido,
    TimestampInvalido,
    EnderecoInvalido,
    TokenNaoEncontrado,
    TransacaoRepetida,
    ValorInvalido,
    InvalidSignature(String),
    InvalidPublicKey(String),
    InvalidData(String),
}


impl From<Error> for TransactionError {
    fn from(err: Error) -> Self {
        TransactionError::OqsError(err)
    }
}


impl std::error::Error for TransactionError {}


impl fmt::Display for TransactionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransactionError::OqsError(e) => write!(f, "OQS error: {}", e),
            TransactionError::InvalidData(msg) => write!(f, "Invalid data: {}", msg),
            TransactionError::InvalidSignature(msg) => write!(f, "Invalid signature: {}", msg),
            TransactionError::InvalidTransaction => write!(f, "Invalid transaction"),
            TransactionError::InvalidDataFormat => write!(f, "Invalid data format"),
            TransactionError::InsufficientFunds => write!(f, "Insufficient funds"),
            TransactionError::NonceInvalido => write!(f, "Invalid nonce"),
            TransactionError::TimestampInvalido => write!(f, "Invalid timestamp"),
            TransactionError::EnderecoInvalido => write!(f, "Invalid address"),
            TransactionError::TokenNaoEncontrado => write!(f, "Token not found"),
            TransactionError::TransacaoRepetida => write!(f, "Duplicate transaction"),
            TransactionError::ValorInvalido => write!(f, "Invalid value"),
            TransactionError::InvalidPublicKey(msg) => write!(f, "Invalid public key: {}", msg),
            
        }
    }
}

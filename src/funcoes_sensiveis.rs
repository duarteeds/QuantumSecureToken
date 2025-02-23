
use crate::quantum_crypto::QuantumCrypto;
use oqs::Error as OqsError;

pub fn validar_dados_sensiveis(data: &[u8]) -> Result<bool, OqsError> {
    let crypto = QuantumCrypto::new()?;
    let (encrypted, cipher, key) = crypto.encrypt(data)?;
    let decrypted = crypto.decrypt(&encrypted, &cipher, &key)?;
    Ok(data == decrypted.as_slice())
}
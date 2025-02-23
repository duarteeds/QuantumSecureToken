use oqs::kem::{Algorithm, Kem};

pub fn criptografar_dados(data: &str) -> Vec<u8> {
    let kem = Kem::new(Algorithm::Kyber512).unwrap();
    let (public_key, _) = kem.keypair().unwrap();
    let (_, shared_secret) = kem.encapsulate(&public_key).unwrap();
    
    data.as_bytes()
        .iter()
        .zip(shared_secret.as_ref().iter().cycle())
        .map(|(a, b)| a ^ b)
        .collect()
}
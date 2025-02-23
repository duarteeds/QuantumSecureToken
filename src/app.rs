use std::path::Path;
use log::info;
use anyhow::{Result, Context};
use rusqlite::params;

use crate::blockchain::Blockchain;
use crate::key_manager::KeyManager;
use crate::database::Database;
use crate::token::Token;
use crate::token::token_builder::TokenBuilder;
use crate::token::custom_token::CustomToken;

pub struct QuantumBlockchainApp {
    pub blockchain: Blockchain,
    pub key_manager: KeyManager,
    pub database: Database,
}

impl QuantumBlockchainApp {
    pub fn new() -> Result<Self> {
        let key_manager = KeyManager::new()
            .context("Falha ao inicializar gerenciador de chaves")?;
            
        let blockchain = if Path::new(crate::BLOCKCHAIN_FILE).exists() {
            let mut blockchain = Blockchain::load_from_file(crate::BLOCKCHAIN_FILE)
                .context("Falha ao carregar blockchain")?;

            // Verifica se o Quantum Secure Token está presente
            if !blockchain.tokens.contains_key(&0.to_string()) {
                blockchain.create_quantum_secure_token()?;
                blockchain.save_to_file(crate::BLOCKCHAIN_FILE)?;
            }

            blockchain
        } else {
            info!("Criando nova blockchain");
            let blockchain = Blockchain::new()
                .context("Falha ao criar nova blockchain")?;
            blockchain.save_to_file(crate::BLOCKCHAIN_FILE)?;
            blockchain
        };

        let database = Database::new(crate::DB_PATH)
            .context("Falha ao inicializar banco de dados")?;

        Ok(Self {
            blockchain,
            key_manager,
            database,
        })
    }

    pub fn create_token(&mut self, name: String, symbol: String, supply: u64) -> Result<Token> {
    let token = TokenBuilder::new()
        .name(name.clone())
        .symbol(symbol.clone())
        .total_supply(supply)
        .creator("admin".to_string())
        .build()
        .context("Falha ao construir token")?;

    let conn = self.database.get_connection_mut()
        .context("Falha ao obter conexão com banco de dados")?;

    // Insere o token no banco de dados (sem passar o ID)
    conn.execute(
        "INSERT INTO tokens (name, symbol, supply, creator) VALUES (?1, ?2, ?3, ?4)",
        params![token.name, token.symbol, token.total_supply, token.creator],
    ).context("Falha ao inserir token no banco de dados")?;

    Ok(token)
}

    pub fn create_custom_token(&mut self, id: u32, name: String, symbol: String, supply: u64, owner: String) -> Result<CustomToken> {
        let mut token = CustomToken::new(id, name.clone(), symbol.clone(), supply, owner.clone())?;
        
        // Assinando a transação de criação
        let data = format!("create_token:{}:{}:{}:{}", name, symbol, supply, owner);
        token.sign_transaction(&data)?;
        
        Ok(token)
    }

    pub fn transfer_token(&mut self, token: &mut CustomToken, to: String, amount: u64) -> Result<()> {
        token.transfer(to.clone(), amount)?;
        
        let conn = self.database.get_connection_mut()
            .context("Falha ao obter conexão com banco de dados")?;
            
        conn.execute(
            "INSERT INTO transfers (token_id, from_address, to_address, amount) VALUES (?1, ?2, ?3, ?4)",
            params![token.id, token.owner, to, amount],
        ).context("Falha ao registrar transferência no banco de dados")?;

        Ok(())
    }

    pub fn verify_chain_integrity(&self) -> Result<bool> {
        self.blockchain.is_chain_valid()
            .context("Falha ao verificar integridade da blockchain")
    }
}
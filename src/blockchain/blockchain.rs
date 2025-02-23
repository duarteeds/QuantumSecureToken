use rusqlite::{Connection, Result as SqlResult, params};
use std::io::{Read, Write};
use std::fs::File;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use super::validacao::{validar_transacao, TransacaoErro};
use oqs::Error as OqsError;
use crate::transaction::{Transaction, TransactionError, SecureTransaction};
use crate::token::Token;
use crate::smart_contract::SmartContract;
use super::block::Block;

pub type Address = String;

#[derive(Debug, Serialize, Deserialize)]
pub struct Blockchain {
    pub chain: Vec<Block>,
    pub tokens: HashMap<String, Token>,  // Changed from u64 to String
    pub stakers: HashMap<Address, u64>,
    pub nonces: HashMap<Address, u64>,
    pub pending_transactions: Vec<Transaction>,
    pub blocks: Vec<Block>,
    pub transactions: Vec<Transaction>,
    pub next_token_id: u64,
}

    impl Blockchain {
    pub fn new() -> anyhow::Result<Self> {
        let mut blockchain = Blockchain {
            tokens: HashMap::new(),
            stakers: HashMap::new(),
            chain: Vec::new(),
            next_token_id: 0,
            nonces: HashMap::new(),
            pending_transactions: Vec::new(),
            blocks: Vec::new(),
            transactions: Vec::new(),
        };

        blockchain.create_quantum_secure_token()?;
        Ok(blockchain)
    }

    /// Cria o Quantum Secure Token (ID = 0).

    pub fn create_quantum_secure_token(&mut self) -> anyhow::Result<()> {
        let quantum_token = Token::new(
            "Quantum Secure Token".to_string(),
            "QST".to_string(),
            1_000_000,  // Supply inicial
            "system".to_string(),  // Criador
        );

        self.tokens.insert(0.to_string(), quantum_token?);  // Usando ? para propagar o erro
        self.next_token_id = 1;  // Próximo token terá ID = 1
        Ok(())
    }

    pub fn get_token(&self, id: &str) -> Option<&Token> {
        self.tokens.get(id)
    }

    /// Cria um novo token (para usuários).

    pub fn create_token(
    &mut self, 
    name: String, 
    symbol: String, 
    initial_supply: u64, 
    creator: String
) -> Result<String, OqsError> {  // Usando OqsError
    let token = Token::new(name, symbol, initial_supply, creator)?;
    let token_id = self.next_token_id.to_string();
    self.tokens.insert(token_id.clone(), token);
    self.next_token_id += 1;
    Ok(token_id)
}

    /// Adiciona uma transação à blockchain.

    pub fn add_transaction(&mut self, from: String, to: String, amount: u64) -> Result<(), TransactionError> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    
    let nonce = self.nonces.get(&from).copied().unwrap_or(0) + 1;
    
    let secure_transaction = SecureTransaction::new(from.clone(), to, amount, timestamp, nonce)?;

    if !secure_transaction.verify()? {
        return Err(TransactionError::InvalidSignature("Signature too large".to_string()));
    }

    self.nonces.insert(from, nonce);
    let transaction: Transaction = secure_transaction.into();
    self.pending_transactions.push(transaction);
    Ok(())
}

    pub fn adicionar_transacao(&mut self, transaction: Transaction) -> Result<(), TransacaoErro> {
        // Obter o nonce atual para o endereço
        let nonce_atual = self.nonces.get(&transaction.from).copied().unwrap_or(0);
        
        // Validar a transação
        validar_transacao(&transaction, self, nonce_atual)?;
        
        // Se a validação passar, adicionar a transação
        self.nonces.insert(transaction.from.clone(), nonce_atual + 1);
        self.pending_transactions.push(transaction);
        
        Ok(())
    }

    /// Adiciona um staker à blockchain.
    pub fn add_staker(&mut self, address: String, amount: u64) {
        let current = self.stakers.entry(address).or_insert(0);
        *current += amount;
    }

    /// Adiciona um bloco à blockchain.
    pub fn add_block(
    &mut self,
    transactions: Vec<Transaction>,
    contracts: Vec<SmartContract>,
) -> Result<(), TransactionError> {
    let secure_transactions: Result<Vec<SecureTransaction>, TransactionError> = transactions
        .into_iter()
        .map(|t| {
            SecureTransaction::new(
                t.from.clone(),
                t.to.clone(),
                t.amount,
                t.timestamp,
                t.nonce
            )
        })
        .collect();

    let secure_transactions = secure_transactions?;

    for transaction in &secure_transactions {
        transaction.verify()?;
    }

    let previous_hash = if self.chain.is_empty() {
        "0".to_string()
    } else {
        self.chain.last().unwrap().hash.clone()
    };

    let new_index = if self.chain.is_empty() {
        0
    } else {
        self.chain.last().unwrap().index + 1
    };

    let new_block = match Block::new(
    new_index,
    secure_transactions,
    contracts,
    previous_hash
) {
    Ok(block) => block,
    Err(_) => return Err(TransactionError::OqsError(OqsError::AlgorithmDisabled)),
};

    self.chain.push(new_block);
    Ok(())
}

    /// Salva a blockchain em um arquivo JSON.
    pub fn save_to_file(&self, filename: &str) -> std::io::Result<()> {
        let json = serde_json::to_string(self)?;
        let mut file = File::create(filename)?;
        file.write_all(json.as_bytes())?;
        Ok(())
    }

    /// Salva a blockchain em um banco de dados SQLite.
    pub fn save_to_db(&self, db_path: &str) -> SqlResult<()> {
        let conn = Connection::open(db_path)?;
        
        conn.execute(
            "CREATE TABLE IF NOT EXISTS blocks (
                id INTEGER PRIMARY KEY,
                index INTEGER NOT NULL,
                timestamp INTEGER NOT NULL,
                previous_hash TEXT NOT NULL,
                hash TEXT NOT NULL
            )",
            [],
        )?;

        for block in &self.chain {
            conn.execute(
                "INSERT INTO blocks (index, timestamp, previous_hash, hash) VALUES (?1, ?2, ?3, ?4)",
                params![
                    block.index,
                    block.timestamp,
                    &block.previous_hash,
                    &block.hash
                ],
            )?;
        }

        Ok(())
    }

    pub fn load_from_db(db_path: &str) -> SqlResult<Self> {
        let conn = Connection::open(db_path)?;
        let mut stmt = conn.prepare("SELECT index, timestamp, previous_hash, hash FROM blocks")?;
        let blocks = stmt.query_map([], |row| {
            Ok(Block {
                index: row.get(0)?,
                timestamp: row.get(1)?,
                transactions: vec![],
                contracts: vec![],
                previous_hash: row.get(2)?,
                hash: row.get(3)?,
                validator_signature: None,
            })
        })?;

        let chain: Vec<Block> = blocks.collect::<SqlResult<_>>()?;

        Ok(Blockchain {
    tokens: HashMap::new(),
    stakers: HashMap::new(),
    chain,
    next_token_id: 0,
    nonces: HashMap::new(),
    pending_transactions: Vec::new(),
    blocks: Vec::new(), // Initialize blocks
    transactions: Vec::new(), // Initialize transactions
})
    }

    /// Carrega a blockchain de um arquivo JSON.
    pub fn load_from_file(filename: &str) -> std::io::Result<Self> {
        let mut file = File::open(filename)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        let blockchain: Blockchain = serde_json::from_str(&contents)?;
        Ok(blockchain)
    }

    /// Verifica se a blockchain é válida.
    pub fn is_chain_valid(&self) -> Result<bool, TransactionError> {
    for i in 1..self.chain.len() {
        let current_block = &self.chain[i];
        let previous_block = &self.chain[i - 1];

        if current_block.previous_hash != previous_block.hash {
            return Ok(false);
        }

        for transaction in &current_block.transactions {
            transaction.verify()?;
        }

        let calculated_hash = match Block::calculate_hash(
    current_block.index,
    current_block.timestamp,
    &current_block.transactions,
    &current_block.contracts,
    &current_block.previous_hash,
) {
    Ok(hash) => hash,
    Err(_) => return Err(TransactionError::OqsError(OqsError::AlgorithmDisabled)),
};

        if current_block.hash != calculated_hash {
            return Ok(false);
        }
    }
    Ok(true)
}

}

    


    
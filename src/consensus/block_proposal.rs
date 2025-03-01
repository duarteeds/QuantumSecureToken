use crate::consensus::reputation::{ReputationAction, ReputationSystem};
use crate::consensus::types::{ConsensusError, VerificationResult};
use crate::consensus::validator::ValidatorSet;
use log::{debug, warn};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

/// Representa uma proposta de bloco no sistema de consenso
#[derive(Debug, Clone)]
pub struct BlockProposal {
    /// Hash do bloco proposto
    pub block_hash: String,

    /// Altura do bloco proposto
    pub block_height: u64,

    /// Hash do bloco pai
    pub parent_hash: String,

    /// Timestamp da proposta (em milissegundos desde epoch)
    pub timestamp: u64,

    /// ID do validador que propôs o bloco
    pub proposer_id: String,

    /// Assinatura Dilithium do proposer
    pub signature: Vec<u8>,

    /// Lista de transações incluídas (apenas hashes)
    pub transaction_hashes: Vec<String>,

    /// Dados extras do consenso (específicos para cada tipo)
    pub consensus_data: Vec<u8>,

    /// Quando a proposta foi recebida localmente
    pub received_at: Instant,
}

impl BlockProposal {
    /// Cria uma nova proposta de bloco
    pub fn new(
        block_hash: String,
        block_height: u64,
        parent_hash: String,
        proposer_id: String,
        transaction_hashes: Vec<String>,
        signature: Vec<u8>,
        consensus_data: Vec<u8>,
    ) -> Self {
        Self {
            block_hash,
            block_height,
            parent_hash,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            proposer_id,
            signature,
            transaction_hashes,
            consensus_data,
            received_at: Instant::now(),
        }
    }

    /// Verifica se a proposta expirou
    pub fn is_expired(&self, timeout: Duration) -> bool {
        self.received_at.elapsed() > timeout
    }

    /// Calcula a idade da proposta
    pub fn age(&self) -> Duration {
        self.received_at.elapsed()
    }

    /// Obtém o número de transações incluídas na proposta
    pub fn transaction_count(&self) -> usize {
        self.transaction_hashes.len()
    }
}

/// Verifica propostas de blocos
pub struct ProposalVerifier {
    /// Conjunto de validadores
    validators: Arc<RwLock<ValidatorSet>>,

    /// Sistema de reputação
    reputation: Arc<RwLock<ReputationSystem>>,
}

impl ProposalVerifier {
    /// Cria um novo verificador de propostas
    pub fn new(
        validators: Arc<RwLock<ValidatorSet>>,
        reputation: Arc<RwLock<ReputationSystem>>,
    ) -> Self {
        Self {
            validators,
            reputation,
        }
    }

    /// Verifica uma proposta de bloco
    pub fn verify(&self, proposal: &BlockProposal) -> Result<VerificationResult, ConsensusError> {
        // Verifica se o proposer é um validador conhecido
        let validators = self.validators.read().map_err(|_| {
            ConsensusError::InternalError(
                "Falha ao obter lock do conjunto de validadores".to_string(),
            )
        })?;

        // Verifica se o proposer existe e está ativo
        if let Some(v) = validators.get_validator(&proposal.proposer_id) {
            if !v.is_active {
                warn!(
                    "Proposta rejeitada: validador {} não está ativo",
                    proposal.proposer_id
                );
                return Ok(VerificationResult::Invalid(
                    "Validador não está ativo".to_string(),
                ));
            }
        // Use v here if needed, e.g., for stake check or signature verification
        } else {
            warn!(
                "Proposta rejeitada: validador {} desconhecido",
                proposal.proposer_id
            );
            return Ok(VerificationResult::Invalid(
                "Validador desconhecido".to_string(),
            ));
        }

        // Verifica a assinatura Dilithium (simplificado - na implementação real verificaria criptograficamente)
        if proposal.signature.is_empty() {
            warn!(
                "Proposta rejeitada: assinatura vazia de {}",
                proposal.proposer_id
            );
            return Ok(VerificationResult::Invalid(
                "Assinatura inválida".to_string(),
            ));
        }

        // Aqui seria inserida a verificação criptográfica da assinatura Dilithium
        // usando a chave pública do validador

        // Verifica o timestamp
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let time_diff = if current_time > proposal.timestamp {
            current_time - proposal.timestamp
        } else {
            proposal.timestamp - current_time
        };

        // Rejeita propostas com timestamp muito divergente (mais de 5 minutos)
        if time_diff > 300_000 {
            warn!(
                "Proposta rejeitada: timestamp divergente por {} ms",
                time_diff
            );
            return Ok(VerificationResult::Invalid(
                "Timestamp inválido".to_string(),
            ));
        }

        // Verifica se o proposer tem stake mínimo necessário
        // (poderia verificar outros requisitos conforme o tipo de consenso)

        // Atualiza a reputação do proposer se a proposta parece válida
        let mut reputation = self.reputation.write().map_err(|_| {
            ConsensusError::InternalError("Falha ao obter lock do sistema de reputação".to_string())
        })?;

        // Registro positivo para o proposer (a reputação final seria atualizada após validação completa do bloco)
        if let Err(e) = reputation
            .update_reputation(&proposal.proposer_id, ReputationAction::ValidBlockProposed)
        {
            debug!("Não foi possível atualizar reputação: {}", e);
        }

        // Proposta considerada válida após todas as verificações
        debug!(
            "Proposta do bloco {} por {} verificada com sucesso",
            proposal.block_height, proposal.proposer_id
        );

        Ok(VerificationResult::Valid)
    }
}

/// Voto em uma proposta de bloco
#[derive(Debug, Clone)]
pub struct ProposalVote {
    /// Hash do bloco sendo votado
    pub block_hash: String,

    /// Altura do bloco
    pub block_height: u64,

    /// ID do validador que está votando
    pub validator_id: String,

    /// Se o voto é a favor (true) ou contra (false)
    pub is_in_favor: bool,

    /// Assinatura Dilithium do voto
    pub signature: Vec<u8>,

    /// Quando o voto foi emitido
    pub timestamp: u64,
}

impl ProposalVote {
    /// Cria um novo voto
    pub fn new(
        block_hash: String,
        block_height: u64,
        validator_id: String,
        is_in_favor: bool,
        signature: Vec<u8>,
    ) -> Self {
        Self {
            block_hash,
            block_height,
            validator_id,
            is_in_favor,
            signature,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        }
    }
}

/// Resultado da votação em uma proposta
#[derive(Debug, Clone)]
pub struct VotingResult {
    /// Hash do bloco votado
    pub block_hash: String,

    /// Altura do bloco
    pub block_height: u64,

    /// Número total de votos recebidos
    pub total_votes: usize,

    /// Número de votos a favor
    pub votes_in_favor: usize,

    /// Número de votos contra
    pub votes_against: usize,

    /// Porcentagem de votos a favor (0-100)
    pub approval_percentage: f32,

    /// Se o resultado atingiu o quórum necessário
    pub reached_quorum: bool,

    /// Se a proposta foi aprovada
    pub is_approved: bool,
}

/// Coordena a votação em propostas de blocos
pub struct VotingCoordinator {
    /// Conjunto de validadores
    validators: Arc<RwLock<ValidatorSet>>,

    /// Sistema de reputação
    reputation: Arc<RwLock<ReputationSystem>>,

    /// Votos recebidos para a proposta atual (hash do bloco -> voto)
    current_votes: HashMap<String, Vec<ProposalVote>>,

    /// Limiar de aprovação (porcentagem necessária para aprovar)
    approval_threshold: f32,
}

impl VotingCoordinator {
    /// Cria um novo coordenador de votação
    pub fn new(
        validators: Arc<RwLock<ValidatorSet>>,
        reputation: Arc<RwLock<ReputationSystem>>,
        approval_threshold: f32,
    ) -> Self {
        Self {
            validators,
            reputation,
            current_votes: HashMap::new(),
            approval_threshold,
        }
    }

    /// Processa um novo voto
    pub fn process_vote(&mut self, vote: ProposalVote) -> Result<bool, ConsensusError> {
        // Verifica se o validador existe e está ativo
        let validators = self.validators.read().map_err(|_| {
            ConsensusError::InternalError(
                "Falha ao obter lock do conjunto de validadores".to_string(),
            )
        })?;

        if !validators.contains(&vote.validator_id) {
            warn!(
                "Voto rejeitado: validador {} desconhecido",
                vote.validator_id
            );
            return Err(ConsensusError::InvalidProposer(format!(
                "Validador desconhecido: {}",
                vote.validator_id
            )));
        }

        // Verifica a assinatura do voto (simplificado)
        // Aqui seria inserida a verificação criptográfica usando Dilithium

        // Verifica se o validador já votou nesta proposta (evita double voting)
        let votes = self
            .current_votes
            .entry(vote.block_hash.clone())
            .or_insert_with(Vec::new);

        if votes.iter().any(|v| v.validator_id == vote.validator_id) {
            warn!(
                "Tentativa de double-voting detectada: {} para bloco {}",
                vote.validator_id, vote.block_hash
            );

            // Penaliza o validador por tentar double-voting
            let mut reputation = self.reputation.write().map_err(|_| {
                ConsensusError::InternalError(
                    "Falha ao obter lock do sistema de reputação".to_string(),
                )
            })?;

            if let Err(e) =
                reputation.update_reputation(&vote.validator_id, ReputationAction::DoubleVote)
            {
                debug!("Não foi possível atualizar reputação: {}", e);
            }

            return Err(ConsensusError::ValidationFailed(
                "Tentativa de double-voting".to_string(),
            ));
        }

        // Adiciona o voto
        votes.push(vote.clone());

        // Atualiza a reputação com base no voto
        // (a correção do voto seria avaliada após a finalização do bloco)

        debug!(
            "Voto de {} para bloco {} registrado: {}",
            vote.validator_id,
            vote.block_hash,
            if vote.is_in_favor {
                "a favor"
            } else {
                "contra"
            }
        );

        Ok(true)
    }

    /// Conta os votos para uma proposta específica
    pub fn tally_votes(&self, block_hash: &str) -> Option<VotingResult> {
        let votes = self.current_votes.get(block_hash)?;

        if votes.is_empty() {
            return None;
        }

        // Contagem de votos
        let total_votes = votes.len();
        let votes_in_favor = votes.iter().filter(|v| v.is_in_favor).count();
        let votes_against = total_votes - votes_in_favor;

        // Cálculo de porcentagem
        let approval_percentage = (votes_in_favor as f32 / total_votes as f32) * 100.0;

        // Verifica quórum e aprovação
        let validators = match self.validators.read() {
            Ok(v) => v,
            Err(_) => return None,
        };

        let total_validators = validators.count_active();
        let reached_quorum = total_votes >= (total_validators * 2 / 3);
        let is_approved = approval_percentage >= self.approval_threshold;

        // Primeira altura de bloco das propostas votadas
        let block_height = votes[0].block_height;

        Some(VotingResult {
            block_hash: block_hash.to_string(),
            block_height,
            total_votes,
            votes_in_favor,
            votes_against,
            approval_percentage,
            reached_quorum,
            is_approved: reached_quorum && is_approved,
        })
    }

    /// Limpa os votos de uma proposta
    pub fn clear_votes(&mut self, block_hash: &str) {
        self.current_votes.remove(block_hash);
    }

    /// Verifica se uma proposta atingiu finalidade
    pub fn has_reached_finality(&self, block_hash: &str) -> bool {
        match self.tally_votes(block_hash) {
            Some(result) => result.is_approved,
            None => false,
        }
    }
}

use std::collections::HashMap;

use log::{debug, info};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::utils::serde_helpers::SerializableInstant;

/// Ações que podem afetar a reputação de um validador
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReputationAction {
    /// Validador propôs um bloco válido
    ValidBlockProposed,

    /// Validador propôs um bloco inválido
    InvalidBlockProposed,

    /// Validador votou corretamente
    CorrectVote,

    /// Validador votou incorretamente
    IncorrectVote,

    /// Validador não respondeu dentro do timeout
    Timeout,

    /// Validador tentou votar duas vezes (comportamento malicioso)
    DoubleVote,

    /// Validador esteve offline quando deveria estar ativo
    Offline,

    /// Validador voltou online após período offline
    BackOnline,

    /// Validador enviou mensagem inválida ou malformada
    InvalidMessage,

    /// Validador tentou propor quando não era sua vez
    UnauthorizedProposal,
}

/// Reputação de um único validador
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorReputation {
    /// ID único do validador
    pub validator_id: String,

    /// Pontuação atual de reputação (0-100)
    pub score: f32,

    /// Número de blocos propostos com sucesso
    pub successful_proposals: u64,

    /// Número de blocos propostos inválidos
    pub invalid_proposals: u64,

    /// Número de votos corretos
    pub correct_votes: u64,

    /// Número de votos incorretos
    pub incorrect_votes: u64,

    /// Número de timeouts (não responder a tempo)
    pub timeouts: u64,

    /// Número de votos duplicados (comportamento malicioso)
    pub double_votes: u64,

    /// Tempo em que o validador foi observado pela última vez
    pub last_seen: SerializableInstant,

    /// Se o validador está atualmente banido
    pub is_banned: bool,

    /// Até quando o validador está banido (se aplicável)
    pub banned_until: Option<SerializableInstant>,
}

impl ValidatorReputation {
    /// Cria um novo registro de reputação para um validador
    pub fn new(validator_id: String) -> Self {
        Self {
            validator_id,
            score: 50.0, // Inicia com reputação neutra
            successful_proposals: 0,
            invalid_proposals: 0,
            correct_votes: 0,
            incorrect_votes: 0,
            timeouts: 0,
            double_votes: 0,
            last_seen: Instant::now().into(),
            is_banned: false,
            banned_until: None,
        }
    }

    /// Atualiza o momento em que o validador foi visto pela última vez
    pub fn update_last_seen(&mut self) {
        self.last_seen = Instant::now().into();
    }

    /// Verifica se o validador está offline com base em um limite de tempo
    pub fn is_offline(&self, threshold: Duration) -> bool {
        self.last_seen.elapsed() > threshold
    }

    /// Bane o validador por um período específico
    pub fn ban(&mut self, duration: Duration) {
        self.is_banned = true;
        self.banned_until = Some((Instant::now() + duration).into());
        self.score = self.score.max(10.0); // Reduz a reputação, mas mantém um mínimo
    }

    /// Verifica se o banimento expirou e atualiza o status
    pub fn check_ban_status(&mut self) -> bool {
        if let Some(until) = &self.banned_until {
            if Instant::now() >= until.to_instant() {
                self.is_banned = false;
                self.banned_until = None;
                return true; // Ban expirou
            }
        }
        false
    }
}

/// Sistema de reputação que gerencia a reputação de todos os validadores
#[derive(Debug, Clone)]
pub struct ReputationSystem {
    /// Mapa de IDs de validadores para suas reputações
    reputations: HashMap<String, ValidatorReputation>,

    /// Configurações de ajuste de reputação
    config: ReputationConfig,
}

/// Configurações para o sistema de reputação
#[derive(Debug, Clone)]
pub struct ReputationConfig {
    /// Pontos ganhos por propor um bloco válido
    pub valid_block_points: f32,

    /// Pontos perdidos por propor um bloco inválido
    pub invalid_block_penalty: f32,

    /// Pontos ganhos por votar corretamente
    pub correct_vote_points: f32,

    /// Pontos perdidos por votar incorretamente
    pub incorrect_vote_penalty: f32,

    /// Pontos perdidos por timeout
    pub timeout_penalty: f32,

    /// Pontos perdidos por voto duplo (severo)
    pub double_vote_penalty: f32,

    /// Pontos perdidos por ficar offline
    pub offline_penalty: f32,

    /// Pontos ganhos por voltar online
    pub back_online_points: f32,

    /// Limiar de pontos abaixo do qual um validador é considerado suspeito
    pub suspicious_threshold: f32,

    /// Limiar de pontos abaixo do qual um validador é banido
    pub ban_threshold: f32,

    /// Duração do banimento inicial (aumenta com reincidências)
    pub initial_ban_duration: Duration,

    /// Fator de decaimento para ajustes negativos repetidos
    pub decay_factor: f32,
}

impl Default for ReputationConfig {
    fn default() -> Self {
        Self {
            valid_block_points: 2.0,
            invalid_block_penalty: 10.0,
            correct_vote_points: 1.0,
            incorrect_vote_penalty: 5.0,
            timeout_penalty: 3.0,
            double_vote_penalty: 20.0,
            offline_penalty: 5.0,
            back_online_points: 1.0,
            suspicious_threshold: 30.0,
            ban_threshold: 15.0,
            initial_ban_duration: Duration::from_secs(3600), // 1 hora
            decay_factor: 0.9,
        }
    }
}

impl ReputationSystem {
    /// Cria um novo sistema de reputação com configurações padrão
    pub fn new() -> Self {
        Self {
            reputations: HashMap::new(),
            config: ReputationConfig::default(),
        }
    }

     pub fn config(&self) -> &ReputationConfig {
        &self.config
    }

    pub fn set_config(&mut self, config: ReputationConfig) {
        self.config = config;
    }

    pub fn ban_validator(&mut self, validator_id: &str, duration: Duration) -> Result<(), String> {
        // Obtém a reputação do validador
        let reputation = self
            .reputations
            .get_mut(validator_id)
            .ok_or_else(|| format!("Validador não encontrado: {}", validator_id))?;

        // Aplica o banimento
        reputation.ban(duration);
        Ok(())
    }

    // Método para obter uma referência mutável à reputação
    pub fn get_reputation_mut(&mut self, validator_id: &str) -> Option<&mut ValidatorReputation> {
        self.reputations.get_mut(validator_id)
    }

    /// Cria um sistema de reputação com configurações personalizadas
    pub fn with_config(config: ReputationConfig) -> Self {
        Self {
            reputations: HashMap::new(),
            config,
        }
    }

    /// Adiciona um novo validador ao sistema
    pub fn add_validator(&mut self, validator_id: String) {
        if !self.reputations.contains_key(&validator_id) {
            let reputation = ValidatorReputation::new(validator_id.clone());
            self.reputations.insert(validator_id.clone(), reputation);
            debug!(
                "Adicionado novo validador ao sistema de reputação: {}",
                validator_id
            );
        }
    }

    /// Atualiza a reputação de um validador com base em uma ação
    pub fn update_reputation(
        &mut self,
        validator_id: &str,
        action: ReputationAction,
    ) -> Result<f32, String> {
        let reputation = self
            .reputations
            .get_mut(validator_id)
            .ok_or_else(|| format!("Validador não encontrado: {}", validator_id))?;

        // Atualiza o timestamp de última atividade
        reputation.update_last_seen();

        // Se estiver banido, verifica se o ban expirou
        if reputation.is_banned {
            reputation.check_ban_status();
            if reputation.is_banned {
                return Err(format!("Validador está banido: {}", validator_id));
            }
        }

        // Aplica o ajuste de reputação com base na ação
        let adjustment = match action {
            ReputationAction::ValidBlockProposed => {
                reputation.successful_proposals += 1;
                self.config.valid_block_points
            }
            ReputationAction::InvalidBlockProposed => {
                reputation.invalid_proposals += 1;
                -self.config.invalid_block_penalty
            }
            ReputationAction::CorrectVote => {
                reputation.correct_votes += 1;
                self.config.correct_vote_points
            }
            ReputationAction::IncorrectVote => {
                reputation.incorrect_votes += 1;
                -self.config.incorrect_vote_penalty
            }
            ReputationAction::Timeout => {
                reputation.timeouts += 1;
                -self.config.timeout_penalty
            }
            ReputationAction::DoubleVote => {
                reputation.double_votes += 1;
                -self.config.double_vote_penalty
            }
            ReputationAction::Offline => -self.config.offline_penalty,
            ReputationAction::BackOnline => self.config.back_online_points,
            ReputationAction::InvalidMessage => -self.config.incorrect_vote_penalty,
            ReputationAction::UnauthorizedProposal => -self.config.invalid_block_penalty,
        };

        // Aplica o ajuste à pontuação
        reputation.score = (reputation.score + adjustment).max(0.0).min(100.0);

        // Verifica se o validador deve ser banido
        if adjustment < 0.0 && reputation.score < self.config.ban_threshold {
            // Calcula um multiplicador baseado nas infrações
            let multiplier = (1.0 + reputation.double_votes as f32)
                * (1.0 + reputation.invalid_proposals as f32);

            let mult_factor = multiplier as u32;
            let ban_duration = self.config.initial_ban_duration * mult_factor;
            reputation.ban(ban_duration);

            info!(
                "Validador {} banido por {:?} devido a pontuação baixa ({:.2})",
                validator_id, ban_duration, reputation.score
            );
        }

        // Registra a mudança de reputação
        if adjustment > 0.0 {
            debug!(
                "Reputação de {} aumentou em {:.2} para {:.2} ({:?})",
                validator_id, adjustment, reputation.score, action
            );
        } else {
            info!(
                "Reputação de {} diminuiu em {:.2} para {:.2} ({:?})",
                validator_id, -adjustment, reputation.score, action
            );
        }

        Ok(reputation.score)
    }

    pub fn set_reputation(&mut self, validator_id: &str, score: f32) -> Result<(), String> {
        let reputation = self
            .reputations
            .get_mut(validator_id)
            .ok_or_else(|| format!("Validador não encontrado: {}", validator_id))?;

        reputation.score = score.max(0.0).min(100.0);
        Ok(())
    }

    /// Obtém a reputação atual de um validador
    pub fn get_reputation(&self, validator_id: &str) -> Option<&ValidatorReputation> {
        self.reputations.get(validator_id)
    }

    /// Retorna todos os validadores com reputação abaixo do limiar de suspeita
    pub fn get_suspicious_validators(&self) -> Vec<&ValidatorReputation> {
        self.reputations
            .values()
            .filter(|rep| rep.score < self.config.suspicious_threshold)
            .collect()
    }

    /// Retorna o número de validadores suspeitos
    pub fn count_suspicious_validators(&self) -> usize {
        self.get_suspicious_validators().len()
    }

    /// Retorna todos os validadores que não estão banidos
    pub fn get_active_validators(&self) -> Vec<&ValidatorReputation> {
        self.reputations
            .values()
            .filter(|rep| !rep.is_banned)
            .collect()
    }

    /// Verifica se um validador está banido
    pub fn is_banned(&self, validator_id: &str) -> bool {
        self.reputations
            .get(validator_id)
            .map(|rep| rep.is_banned)
            .unwrap_or(false)
    }

    /// Atualiza o status de validadores offline com base em um limiar de tempo
    pub fn update_offline_status(&mut self, offline_threshold: Duration) {
        // Primeiro, coletamos os IDs dos validadores offline
        let ids_to_update: Vec<String> = self
            .reputations
            .keys()
            .filter(|id| {
                if let Some(rep) = self.reputations.get(*id) {
                    rep.is_offline(offline_threshold) && !rep.is_banned
                } else {
                    false
                }
            })
            .cloned()
            .collect();

        // Depois, processamos cada ID separadamente
        for id in ids_to_update {
            let _ = self.update_reputation(&id, ReputationAction::Offline);
        }
    }

    /// Retorna a pontuação média de reputação de todos os validadores
    pub fn average_reputation(&self) -> f32 {
        if self.reputations.is_empty() {
            return 50.0; // Valor neutro padrão
        }

        let sum: f32 = self.reputations.values().map(|rep| rep.score).sum();

        sum / self.reputations.len() as f32
    }
}

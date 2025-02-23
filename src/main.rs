use std::error::Error;
use std::fs;
use log::{info, error};
use anyhow::{Result, Context};
use simplelog::*;
use time::macros::format_description;



// Use crate:: em vez de blockchain::
use quantum_blockchain::QuantumBlockchainApp;

fn setup_logging() -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    let config = ConfigBuilder::new()
        .set_time_format_custom(format_description!("%Y-%m-%d %H:%M:%S"))
        .build();
        
    CombinedLogger::init(vec![
        TermLogger::new(
            LevelFilter::Info,
            config.clone(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        ),
        WriteLogger::new(
            LevelFilter::Debug,
            config,
            fs::File::create("blockchain.log")?,
        ),
    ])?;
    
    Ok(())
}

fn main() -> Result<()> {
    if let Err(e) = setup_logging() {
        eprintln!("Erro ao configurar logging: {}", e);
    }
    
    info!("Iniciando aplicação blockchain quântica");
    
    let mut app = QuantumBlockchainApp::new()
        .context("Falha ao inicializar aplicação")?;

    // Criar um token normal
    let token = app.create_token(
        "MyToken".to_string(),
        "MTK".to_string(),
        1_000_000,
    )?;
    
    info!("Token criado: {} ({})", token.name, token.symbol);

    // Criar um token personalizado
    let mut custom_token = app.create_custom_token(
        1,
        "CustomToken".to_string(),
        "CTK".to_string(),
        500_000,
        "0x123...".to_string(),
    )?;
    
    info!("Token personalizado criado: {} ({})", custom_token.name, custom_token.symbol);

    // Transferir tokens
    app.transfer_token(&mut custom_token, "0x456...".to_string(), 1000)?;
    
    info!("Transferência realizada com sucesso");

    if !app.verify_chain_integrity()? {
        error!("Detectada violação de integridade na blockchain!");
        return Err(anyhow::anyhow!("Violação de integridade detectada"));
    }
    
    info!("Aplicação iniciada com sucesso");
    
    Ok(())
}
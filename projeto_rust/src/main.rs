mod explicacoes;
mod bronze;
mod silver;
mod eda;
mod gold;
mod ml;
mod timing;

use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    println!("╔════════════════════════════════════════════════════╗");
    println!("║   PIPELINE DE CIÊNCIA DE DADOS EM RUST + POLARS   ║");
    println!("║   Análise de Incidentes de Cibersegurança          ║");
    println!("║   Arquitetura Medallion: Bronze → Prata → Ouro    ║");
    println!("╚════════════════════════════════════════════════════╝");
    println!();
    println!("👤 Aluno: Benjamin Yuji Suzuki");
    println!("📚 Disciplina: Ciência de Dados");
    println!("🦀 Implementação: Rust com Polars e SmartCore");
    println!();

    let mut t = timing::Timer::new();

    t.begin("Bronze");
    explicacoes::etapa1_bronze();
    bronze::executar()?;
    t.end();

    t.begin("Prata");
    explicacoes::etapa2_prata();
    silver::executar()?;
    t.end();

    t.begin("EDA");
    explicacoes::etapa3_eda();
    eda::executar()?;
    t.end();

    t.begin("Ouro");
    explicacoes::etapa4_ouro();
    gold::executar()?;
    t.end();

    t.begin("ML");
    explicacoes::etapa5_ml();
    ml::executar()?;
    t.end();

    t.save_csv();

    println!("╔════════════════════════════════════════════════════╗");
    println!("║   ✅ PIPELINE COMPLETO COM SUCESSO!               ║");
    println!("╠════════════════════════════════════════════════════╣");
    println!("║   📁 camada_bronze/  → Dados Brutos em Parquet    ║");
    println!("║   📁 camada_prata/   → Dados Limpos e Integrados  ║");
    println!("║   📁 camada_ouro/    → Dados ML-Ready (transform.)║");
    println!("║   📁 graficos/       → 7 gráficos em SVG + 1 fi   ║");
    println!("║   📁 previsoes/      → CSVs de predição           ║");
    println!("║   📁 modelos/        → Modelos serializados (JSON)║");
    println!("╠════════════════════════════════════════════════════╣");
    println!("║   🔬 3 Hipóteses testadas na EDA                  ║");
    println!("║   🤖 2 Modelos de Árvore de Decisão treinados     ║");
    println!("║   ⚖️  Comparação PRATA vs OURO realizada          ║");
    println!("╚════════════════════════════════════════════════════╝");

    Ok(())
}

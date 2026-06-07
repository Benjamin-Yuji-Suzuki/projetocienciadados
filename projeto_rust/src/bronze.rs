use polars::prelude::*;
use chrono::Utc;
use uuid::Uuid;
use std::error::Error;
use std::fs::{self, File};

pub fn executar() -> Result<(), Box<dyn Error>> {
    println!(" [BRONZE] Iniciando ingestao de dados...\n");

    fs::create_dir_all("camada_bronze")?;

    let arquivos = vec![
        ("financial_impact.csv", "camada_inicial/financial_impact.csv"),
        ("incidents_master.csv", "camada_inicial/incidents_master.csv"),
        ("market_impact.csv", "camada_inicial/market_impact.csv"),
    ];

    for (nome, caminho) in &arquivos {
        println!("   Lendo: {}", caminho);

        // --- ESTRATEGIA: CAMADA BRONZE ---
        // Objetivo: Preservar os dados em estado bruto (imutavel) em formato
        // colunar (Parquet) que e mais eficiente que CSV para leituras futuras.
        // Adicionamos 4 colunas de auditoria para rastreabilidade:
        //   1. meta_arquivo_origem: qual CSV gerou este registro
        //   2. meta_qtd_linhas: quantas linhas o arquivo original tinha
        //   3. meta_hash_ingestao: UUID unico deste lote de carga
        //   4. meta_data_carga: timestamp ISO 8601 de quando foi ingerido
        // Isso permite saber EXATAMENTE a procedencia de cada registro.
        let df = CsvReader::new(std::fs::File::open(caminho)?)
            .finish()?;

        let linhas = df.height();
        let timestamp_carga = Utc::now().to_rfc3339();
        let hash_lote = Uuid::new_v4().to_string();

        let mut df_bronze = df.lazy()
            .with_columns([
                lit(nome.to_string()).alias("meta_arquivo_origem"),
                lit(linhas as u32).alias("meta_qtd_linhas"),
                lit(hash_lote).alias("meta_hash_ingestao"),
                lit(timestamp_carga.clone()).alias("meta_data_carga"),
            ])
            .collect()?;

        let destino = format!("camada_bronze/{}.parquet", nome.replace(".csv", ""));
        let mut arquivo = File::create(&destino)?;
        ParquetWriter::new(&mut arquivo).finish(&mut df_bronze)?;

        println!("   {} -> {} ({} linhas)", nome, destino, linhas);
    }

    // Relatorio resumido da Bronze
    println!("\nRELATORIO DA CAMADA BRONZE:");
    println!("   ---------------------------");
    println!("   Tabela          | Linhas | Colunas");
    println!("   ---------------------------");
    for (nome, _) in &arquivos {
        let nome_parquet = format!("camada_bronze/{}.parquet", nome.replace(".csv", ""));
        let mut lf = LazyFrame::scan_parquet(&nome_parquet, ScanArgsParquet::default())?;
        let df = lf.clone().collect()?;
        let schema = lf.collect_schema()?;
        println!("   {:<16} | {:>6} | {:>3}", nome.replace(".csv", ""), df.height(), schema.len());
    }
    println!("   ---------------------------");
    println!("   [BRONZE] Concluida!\n");

    Ok(())
}

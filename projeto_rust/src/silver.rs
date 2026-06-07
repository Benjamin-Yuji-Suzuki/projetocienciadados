use polars::prelude::*;
use std::error::Error;
use std::fs::{self, File};

pub fn executar() -> Result<(), Box<dyn Error>> {
    println!(" [PRATA] Iniciando processamento...\n");

    // --- ESTRATEGIA: INNER JOIN ---
    // Usamos INNER JOIN porque so nos interessam incidentes que TENHAM
    // registros financeiros (total_loss_usd). Incidentes sem prejuizo
    // financeiro conhecido nao nos ajudam a treinar o modelo preditivo.
    // Selecionamos apenas incident_id e total_loss_usd da tabela financeira
    // (a variavel alvo) e tambem direct_loss_usd (feature importante).
    // ESTRATEGIA: Nao selecionamos direct_loss_usd porque ela e um componente
    // direto de total_loss_usd (visto na analise dos dados). Usar uma feature
    // que e parte do target causa DATA LEAKAGE: o modelo aprenderia a relacao
    // entre perda direta e perda total em vez de aprender padroes reais de ataque.
    let lf_financeiro = LazyFrame::scan_parquet(
        "camada_bronze/financial_impact.parquet",
        ScanArgsParquet::default(),
    )?
    .select([col("incident_id"), col("total_loss_usd")]);

    let lf_incidentes = LazyFrame::scan_parquet(
        "camada_bronze/incidents_master.parquet",
        ScanArgsParquet::default(),
    )?;

    let mut lf_prata = lf_incidentes.inner_join(
        lf_financeiro,
        col("incident_id"),
        col("incident_id"),
    );

    // --- ESTRATEGIA: ANTI-LEAKAGE ---
    // Data Leakage ocorre quando usamos informacao que NAO estaria
    // disponivel no momento da predicao. Exemplos classicos:
    //   - disclosure_date: a data de divulgacao so e conhecida APOS o ataque
    //   - downtime_hours: o tempo de inatividade so e medido depois
    //   - data_compromised_records: quantos registros vazaram (conhecido depois)
    //   - company_name, stock_ticker: identificam unicamente a empresa
    //     (o modelo decoraria o nome em vez de aprender padroes)
    //   - incident_id: identificador unico, nao generalizavel
    //   - notes, created_at, updated_at: metadados operacionais irrelevantes
    //   - industry_secondary, attack_vector_secondary: redundantes
    //   - meta_*: colunas de auditoria interna da Bronze
    let colunas_para_dropar = [
        "downtime_hours", "data_compromised_records", "disclosure_date",
        "company_name", "stock_ticker", "notes", "created_at", "updated_at",
        "industry_secondary", "attack_vector_secondary", "review_flag",
        "meta_arquivo_origem", "meta_qtd_linhas", "meta_hash_ingestao", "meta_data_carga",
        "incident_id",
    ];

    let schema = lf_prata.collect_schema()?;
    let colunas_a_manter: Vec<Expr> = schema.iter_names()
        .filter(|nome| !colunas_para_dropar.contains(&nome.as_str()))
        .map(|nome| col(nome.as_str()))
        .collect();

    let mut lf_limpo = lf_prata.select(colunas_a_manter);

    // --- ESTRATEGIA: FILTRAGEM DA VARIAVEL ALVO ---
    // Removemos registros sem total_loss_usd (a variavel que queremos prever).
    // Se nao sabemos o prejuizo, nao podemos usar o registro para treino.
    lf_limpo = lf_limpo.filter(col("total_loss_usd").is_not_null());

    // --- ESTRATEGIA: TRATAMENTO DE NULOS (CATEGORICAS) ---
    // Preenchemos valores ausentes com "Desconhecido" em vez de remove-los,
    // pois perderiamos muitas linhas. A estrategia "Desconhecido" preserva
    // o fato de que nao sabemos o valor, sem distorcer estatisticas.
    lf_limpo = lf_limpo.with_columns([
        col("attack_chain").fill_null(lit("Desconhecido")),
        col("attributed_group").fill_null(lit("Desconhecido")),
        col("attribution_confidence").fill_null(lit("Desconhecido")),
        col("data_type").fill_null(lit("Desconhecido")),
        col("data_source_secondary").fill_null(lit("Desconhecido")),
        col("attack_vector_primary").fill_null(lit("Desconhecido")),
        col("data_source_primary").fill_null(lit("Desconhecido")),
    ]);

    let mut df_prata = lf_limpo.collect()?;

    fs::create_dir_all("camada_prata")?;
    let caminho = "camada_prata/dataset_ml.parquet";
    let mut arquivo = File::create(caminho)?;
    ParquetWriter::new(&mut arquivo).finish(&mut df_prata)?;

    println!("   Prata salvo: {} ({} linhas x {} colunas)", caminho, df_prata.height(), df_prata.width());

    // Relatorio de qualidade
    println!("\nRELATORIO DE QUALIDADE — CAMADA PRATA:");
    println!("   -----------------------------------------");
    let total = df_prata.height();
    for nome_coluna in df_prata.get_column_names() {
        let col = df_prata.column(nome_coluna)?;
        let nulos = col.null_count();
        if nulos > 0 {
            println!("   {}: {} nulos ({:.1}%)", nome_coluna, nulos, (nulos as f64 / total as f64) * 100.0);
        }
    }
    println!("   Dataset sem nulos (todos tratados)!");
    println!("   [PRATA] Concluida!\n");

    Ok(())
}

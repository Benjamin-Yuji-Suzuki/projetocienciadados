pub fn etapa1_bronze() {
    println!("\n═══════════════════════════════════════════════════");
    println!("  ETAPA 1: CAMADA BRONZE — INGESTAO DE DADOS BRUTOS");
    println!("═══════════════════════════════════════════════════\n");
    println!("O QUE ESTAMOS FAZENDO?");
    println!("   A Camada Bronze e a primeira camada da arquitetura Medallion.");
    println!("   Aqui, os dados sao ingeridos exatamente como estao nos arquivos");
    println!("   CSV originais, sem qualquer transformacao ou limpeza.");
    println!();
    println!("PROCESSO:");
    println!("   1. Le cada arquivo CSV da pasta 'camada_inicial/'");
    println!("   2. Converte para o formato PARQUET (colunar, eficiente)");
    println!("   3. Adiciona METADADOS DE AUDITORIA (origem, timestamp, hash)");
    println!();
    println!("POR QUE ISSO E IMPORTANTE?");
    println!("   - Preservamos os dados originais imutaveis (rastreabilidade)");
    println!("   - Parquet e 10x mais rapido que CSV para leitura");
    println!("   - Metadados permitem saber QUANDO e DE ONDE os dados vieram");
    println!();
    println!("DADOS DE ENTRADA:");
    println!("   - financial_impact.csv  -> 778 incidentes com perdas financeiras");
    println!("   - incidents_master.csv  -> 850 registros de ataques ciberneticos");
    println!("   - market_impact.csv     -> 358 registros de impacto no mercado");
    println!();
}

pub fn etapa2_prata() {
    println!("\n═══════════════════════════════════════════════════");
    println!("  ETAPA 2: CAMADA PRATA (SILVER) — LIMPEZA E INTEGRACAO");
    println!("═══════════════════════════════════════════════════\n");
    println!("O QUE ESTAMOS FAZENDO?");
    println!("   A Camada Prata e onde os dados sao limpos, padronizados");
    println!("   e integrados. Unimos tabelas, removemos sujeira e");
    println!("   eliminamos colunas que causariam DATA LEAKAGE.");
    println!();
    println!("PROCESSO:");
    println!("   1. JOIN: Unimos incidents_master + financial_impact pelo incident_id");
    println!("   2. ANTI-LEAKAGE: Removemos colunas que 'vazam' informacao do futuro:");
    println!("      - disclosure_date -> data de divulgacao (so sabemos depois)");
    println!("      - downtime_hours -> indisponibilidade (so apos o incidente)");
    println!("      - company_name, stock_ticker -> identificadores");
    println!("      - notes, created_at, updated_at -> metadados irrelevantes");
    println!("   3. TRATAMENTO DE NULOS: Preenchemos com 'Desconhecido'");
    println!("   4. FILTRAGEM: Removemos registros sem total_loss_usd");
    println!();
    println!("DATA LEAKAGE — CONCEITO CRITICO:");
    println!("   Data Leakage acontece quando usamos informacao que nao");
    println!("   estaria disponivel na hora da predicao. Ex: usar a data");
    println!("   de divulgacao do ataque para prever o prejuizo e trapacear!");
    println!("   Na vida real, voce nao sabe o futuro quando faz a predicao.");
    println!();
}

pub fn etapa3_eda() {
    println!("\n═══════════════════════════════════════════════════");
    println!("  ETAPA 3: EDA — ANALISE EXPLORATORIA DE DADOS");
    println!("═══════════════════════════════════════════════════\n");
    println!("O QUE ESTAMOS FAZENDO?");
    println!("   Exploramos os dados para encontrar padroes, tendencias");
    println!("   e anomalias. Cada hipotese e testada com um grafico.");
    println!();
    println!("HIPOTESES TESTADAS:");
    println!("   H1: Industrias de tecnologia (51) sofrem mais ataques");
    println!("   H2: Ransomware causa o maior prejuizo financeiro medio");
    println!("   H3: Dados mistos (mixed) sao os mais visados por atacantes");
    println!();
    println!("GRAFICOS GERADOS (SVG):");
    println!("   1. grafico1_top_industrias.svg  -> Distrib. por setor");
    println!("   2. grafico2_prejuizo_vetor.svg   -> Prejuizo por ataque");
    println!("   3. grafico3_tipos_dados.svg      -> Dados mais roubados");
    println!("   4. grafico4_histograma_perdas.svg -> Distribuicao de perdas");
    println!("   5. grafico5_outliers.svg         -> Outliers (IQR)");
    println!("   6. grafico6_matriz_correlacao.svg -> Correlacoes numericas");
    println!("   7. feature_importance.svg        -> Importancia Gini");
    println!();
    println!("POR QUE ISSO IMPORTA?");
    println!("   EDA orientada a hipoteses nao e 'desenhar por desenhar'.");
    println!("   Cada grafico deve responder uma pergunta de negocio.");
    println!("   As conclusoes guiam as decisoes de engenharia (Gold) e ML.");
    println!();
}

pub fn etapa4_ouro() {
    println!("\n═══════════════════════════════════════════════════");
    println!("  ETAPA 4: CAMADA OURO (GOLD) — ML-READY");
    println!("═══════════════════════════════════════════════════\n");
    println!("O QUE ESTAMOS FAZENDO?");
    println!("   Preparamos os dados para os modelos de Machine Learning.");
    println!("   Aplicamos encoding, scaling, tratamento de missing e outliers.");
    println!("   Usamos padrao FIT/TRANSFORM para evitar data leakage!");
    println!();
    println!("TRANSFORMACOES:");
    println!("   1. ENCODING (2 tecnicas):");
    println!("      - Label Encoding: confidence_tier (1,2,3,4 -> ordinal)");
    println!("      - One-Hot Encoding: industry_primary (categorias nominais)");
    println!("   2. SCALING:");
    println!("      - StandardScaler: normaliza receita e funcionarios");
    println!("   3. MISSING VALUES (2 estrategias):");
    println!("      - Mediana: para variaveis numericas (robusta a outliers)");
    println!("      - 'Desconhecido': para variaveis categoricas");
    println!("   4. OUTLIERS (2 colunas):");
    println!("      - IQR: total_loss_usd (cap no limite superior)");
    println!("      - Z-score: company_revenue_usd (cap em |z| < 3)");
    println!();
    println!("PADRAO FIT/TRANSFORM:");
    println!("   FIT = aprender parametros (media, desvio, limites) com TREINO");
    println!("   TRANSFORM = aplicar parametros APRENDIDOS no TREINO E TESTE");
    println!("   Isso garante que o modelo NUNCA veja informacoes do teste!");
    println!();
}

pub fn etapa5_ml() {
    println!("\n═══════════════════════════════════════════════════");
    println!("  ETAPA 5: MODELAGEM — ARVORES DE DECISAO");
    println!("═══════════════════════════════════════════════════\n");
    println!("O QUE ESTAMOS FAZENDO?");
    println!("   Treinamos 2 modelos de Arvore de Decisao com configuracoes");
    println!("   diferentes para classificar se um incidente tera ALTO IMPACTO");
    println!("   (prejuizo acima da mediana) ou BAIXO IMPACTO.");
    println!();
    println!("MODELOS:");
    println!("   Modelo 1: Gini Index, max_depth=5 (mais simples)");
    println!("   Modelo 2: Entropy, max_depth=10 (mais complexo)");
    println!();
    println!("METRICAS:");
    println!("   - Acuracia  -> (VP + VN) / Total");
    println!("   - Precisao  -> VP / (VP + FP)");
    println!("   - Recall    -> VP / (VP + FN)");
    println!("   - F1-Score  -> Media harmonica Precisao x Recall");
    println!("   - Matriz de Confusao (SVG)");
    println!();
    println!("SAIDAS ADICIONAIS:");
    println!("   - previsoes/previsoes_modelo1.csv  -> Predicoes do Gini");
    println!("   - previsoes/previsoes_modelo2.csv  -> Predicoes do Entropy");
    println!("   - previsoes/comparacao_modelos.csv -> Modelos lado a lado");
    println!("   - previsoes/feature_importance.csv -> Importancia Gini");
    println!("   - graficos/feature_importance.svg  -> Grafico de importancia");
    println!("   - modelos/modelo_*.json             -> Modelos serializados");
    println!();
    println!("COMPARACAO PRATA vs OURO:");
    println!("   Testamos os modelos com dados da Camada PRATA (cru)");
    println!("   e da Camada OURO (tratado). A diferenca no F1-score");
    println!("   mostra o IMPACTO DO PRE-PROCESSAMENTO!");
    println!();
}

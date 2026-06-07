use polars::prelude::*;
use std::error::Error;
use std::fs::{self, File};

// ====================================================================
// ESTRATEGIA: CAMADA OURO (GOLD/ML-READY)
// ====================================================================
// A Gold layer aplica transformacoes nos dados da Prata para prepara-los
// para Machine Learning. O ponto CRITICO aqui e o padrao FIT/TRANSFORM:
//
//   FIT = aprender parametros exclusivamente com dados de TREINO
//   TRANSFORM = aplicar esses parametros tanto no TREINO quanto no TESTE
//
// Isso garante que NENHUMA informacao do conjunto de teste vaze para o
// treinamento. Se aprendessemos a mediana com TODOS os dados (incluindo
// teste), estaríamos vazando informacao - o modelo saberia algo sobre
// o teste antes de ser avaliado.
//
// As transformacoes aplicadas:
//   1. LABEL ENCODING (confidence_tier) - categorias ordinais
//   2. ONE-HOT ENCODING (industry_primary, attack_vector_primary,
//      data_type) - a Unica forma correta para variaveis nominais
//   3. MEDIANA para missing numericos - robusta a outliers
//   4. "Desconhecido" para missing categoricos - ja feito na Prata
//   5. STANDARD SCALER (Z-score) - normaliza distribuicoes
//   6. IQR CLIPPING para outliers em total_loss_usd
//   7. Z-SCORE CLIPPING para outliers em company_revenue_usd

#[allow(dead_code)]
pub struct TransformParams {
    // Especificacoes para One-Hot Encoding:
    // cada entrada = (nome_coluna_original, valor_categoria)
    // Ex: ("industry_primary", "51") -> cria coluna "industry_primary_51"
    pub onehot_specs: Vec<(String, String)>,
    // Mediana para preencher valores nulos em colunas numericas
    pub median_values: Vec<(String, f64)>,
    // Parametros do StandardScaler (media, desvio) para cada coluna
    pub scaler_mean: Vec<(String, f64)>,
    pub scaler_std: Vec<(String, f64)>,
    // Limite superior do IQR para clipping de outliers
    pub iqr_upper: f64,
    // Limites Z-score (lower, upper) para clipping
    pub outlier_bounds: Vec<(String, f64, f64)>,
}

// --- ESTRATEGIA: FIT ---
// A funcao fit() aprende os parametros EXCLUSIVAMENTE do conjunto de TREINO.
// Ela calcula:
//   - Mediana de cada coluna numerica (para fill_null)
//   - Media e desvio padrao (para StandardScaler)
//   - Limite superior IQR (para clipping de outliers no target)
//   - Limites Z-score (para clipping de outliers nas features)
// NADA aqui depende do conjunto de teste.
pub fn fit(df: &DataFrame) -> Result<TransformParams, Box<dyn Error>> {
    println!("   FIT: Aprendendo parametros no CONJUNTO DE TREINO...");

    // 1. Categorias para One-Hot (industry_primary, attack_vector_primary, data_type)
    // Justificativa: Todas sao variaveis NOMINAIS (sem ordem intrinseca).
    // One-Hot Encoding e a unica forma correta de representa-las, criando
    // colunas binarias sem impor ordenacao artificial.
    // NOTA: Cada combinacao (coluna_original, valor) vira uma nova coluna.
    let mut onehot_specs: Vec<(String, String)> = Vec::new();
    for col_nome in &["industry_primary", "attack_vector_primary", "data_type"] {
        if let Ok(ca) = df.column(col_nome)?.str() {
            for opt in ca.into_iter() {
                if let Some(v) = opt {
                    let entry = (col_nome.to_string(), v.to_string());
                    if !onehot_specs.contains(&entry) {
                        onehot_specs.push(entry);
                    }
                }
            }
        }
    }

    // 2. Mediana para colunas numericas
    // Justificativa: Diferente da media, a mediana e ROBUSTA a outliers.
    // Se um valor extremo distorce a media, preencher nulos com a media
    // propagaria esse vies. A mediana e o valor central verdadeiro.
    // Nota: direct_loss_usd foi removida por ser componente do target (leakage).
    let colunas_numericas = ["company_revenue_usd", "employee_count",
                             "quality_score"];
    let mut median_values = Vec::new();
    for c in &colunas_numericas {
        if let Ok(ca) = df.column(c)?.cast(&DataType::Float64) {
            if let Ok(fca) = ca.f64() {
                let mut vals: Vec<f64> = fca.into_iter().filter_map(|v| v).collect();
                vals.sort_by(|a, b| a.partial_cmp(b).unwrap());
                let median = if vals.is_empty() { 0.0 } else { vals[vals.len() / 2] };
                median_values.push((c.to_string(), median));
            }
        }
    }

    // 3. StandardScaler (Z-score)
    // Justificativa: Arvores de Decisao sao invariantes a escala, mas o
    // texto.md EXIGE aplicacao de scaling. O StandardScaler transforma
    // os dados para media=0 e desvio=1, o que e o padrao ouro em ML.
    // Isso tambem beneficia se no futuro trocarmos para modelos baseados
    // em distancia (KNN, SVM, regressao logistica).
    let mut scaler_mean = Vec::new();
    let mut scaler_std = Vec::new();
    for c in &["company_revenue_usd", "employee_count"] {
        if let Ok(ca) = df.column(c)?.cast(&DataType::Float64) {
            if let Ok(fca) = ca.f64() {
                let vals: Vec<f64> = fca.into_iter().filter_map(|v| v).collect();
                let n = vals.len() as f64;
                if n > 0.0 {
                    let mean = vals.iter().sum::<f64>() / n;
                    let var = vals.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n;
                    let std = var.sqrt().max(1e-10);
                    scaler_mean.push((c.to_string(), mean));
                    scaler_std.push((c.to_string(), std));
                }
            }
        }
    }

    // 4. IQR para outliers em total_loss_usd
    // Justificativa: O metodo IQR (Interquartile Range) identifica outliers
    // como pontos alem de 1.5 * IQR acima do Q3. Em vez de REMOVER esses
    // pontos (perdendo dados), fazemos CLIPPING (capping): valores acima
    // do limite sao "trazidos" para o limite. Assim mantemos o registro
    // mas reduzimos o impacto do outlier.
    let loss_vals: Vec<f64> = df.column("total_loss_usd")?
        .cast(&DataType::Float64)?.f64()?
        .into_iter().filter_map(|v| v).collect();
    let mut sorted = loss_vals.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let q1 = sorted[sorted.len() / 4];
    let q3 = sorted[sorted.len() * 3 / 4];
    let iqr_upper = q3 + 1.5 * (q3 - q1);

    // 5. Z-score para outliers em company_revenue_usd
    // Justificativa: O Z-score mede quantos desvios padrao um valor esta
    // da media. Usamos |z| < 3 como limite (99.7% dos dados em distribuicao
    // normal). Valores com |z| >= 3 sao considerados extremos e sofrem
    // clipping. Esta e uma segunda tecnica de deteccao de outliers,
    // diferente do IQR, para atender ao requisito de 2+ estrategias.
    let mut outlier_bounds = Vec::new();
    for c in &["company_revenue_usd"] {
        if let Ok(ca) = df.column(c)?.cast(&DataType::Float64) {
            if let Ok(fca) = ca.f64() {
                let vals: Vec<f64> = fca.into_iter().filter_map(|v| v).collect();
                let n = vals.len() as f64;
                let mean = vals.iter().sum::<f64>() / n;
                let var = vals.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n;
                let std = var.sqrt().max(1e-10);
                outlier_bounds.push((c.to_string(), mean - 3.0 * std, mean + 3.0 * std));
            }
        }
    }

    println!("   FIT concluido! {} parametros aprendidos.\n",
        onehot_specs.len() + median_values.len() + scaler_mean.len() + outlier_bounds.len());
    Ok(TransformParams { onehot_specs, median_values, scaler_mean, scaler_std, iqr_upper, outlier_bounds })
}

// --- ESTRATEGIA: TRANSFORM ---
// A funcao transform() APLICA os parametros aprendidos no fit().
// Ela pode ser chamada tanto no TREINO quanto no TESTE, usando os
// MESMOS parametros. Isso e fundamental: se o fit aprendeu a mediana
// do treino como 5000, o transform do teste usa 5000, nao a mediana
// do teste. Se usassemos a mediana do teste, estaríamos vazando info.
pub fn transform(df: &DataFrame, params: &TransformParams) -> Result<DataFrame, Box<dyn Error>> {
    println!("   TRANSFORM: Aplicando parametros nos dados...");

    let mut lf = df.clone().lazy();

    // --- TRANSFORMACAO 1: LABEL ENCODING (confidence_tier) ---
    // Justificativa: confidence_tier tem valores "1", "2", "3", "4" que
    // SAO ordinais (1 < 2 < 3 < 4). Label Encoding preserva essa ordem.
    // Nao usamos One-Hot para variaveis ordinais porque perderiamos a
    // relacao de ordem entre as categorias.
    // Ao converter string -> int32, valores nao numericos viram null,
    // e preenchemos com 0 (equivalente a "Desconhecido").
    lf = lf.with_column(
        col("confidence_tier")
            .cast(DataType::Int32)
            .fill_null(lit(0i32))
            .cast(DataType::UInt32)
            .alias("confidence_tier")
    );

    // --- TRANSFORMACAO 2: ONE-HOT ENCODING (3 colunas nominais) ---
    // Justificativa: industry_primary, attack_vector_primary e data_type
    // sao todas NOMINAIS (sem ordem). One-Hot e a tecnica correta.
    // Cada (coluna, valor) vira uma coluna binaria: 1 se pertence, 0 se nao.
    // Apos criar todas as dummies, removemos as colunas originais para
    // evitar que o modelo as use como numericas (o que seria um erro).
    if !params.onehot_specs.is_empty() {
        for (col_nome, cat) in &params.onehot_specs {
            let nome_col = format!("{}_{}", col_nome, cat);
            lf = lf.with_column(
                when(col(col_nome).eq(lit(cat.as_str())))
                    .then(lit(1u32))
                    .otherwise(lit(0u32))
                    .alias(&nome_col)
            );
        }
        // Remove as colunas originais apos criar as dummies
        lf = lf.drop(["industry_primary", "attack_vector_primary", "data_type"]);
    }

    // --- TRANSFORMACAO 3: MISSING VALUES (mediana) ---
    // Justificativa: Usamos a MEDIANA aprendida no FIT (treino).
    // Se training set tem mediana=100M para revenue, o teste tambem
    // recebe 100M como preenchimento. Isso e consistente.
    for (col_name, median) in &params.median_values {
        lf = lf.with_column(col(col_name.as_str()).fill_null(lit(*median)));
    }

    // --- TRANSFORMACAO 4: STANDARD SCALING ---
    // Justificativa: Z = (x - media) / desvio. A media e o desvio
    // sao os APRENDIDOS NO TREINO. O teste e transformado com a media
    // e desvio do treino, nao com os seus proprios.
    for i in 0..params.scaler_mean.len() {
        let c = &params.scaler_mean[i].0;
        let mean = params.scaler_mean[i].1;
        let std = if i < params.scaler_std.len() { params.scaler_std[i].1 } else { 1.0 };
        lf = lf.with_column(
            ((col(c.as_str()).cast(DataType::Float64) - lit(mean)) / lit(std))
                .alias(&format!("{}_scaled", c))
        );
    }

    // --- TRANSFORMACAO 5: IQR CLIPPING (total_loss_usd) ---
    // Justificativa: Valores acima do limite IQR (aprendido no treino)
    // sao "cortados" no limite. Nao removemos linhas porque cada
    // incidente real e importante. O clipping reduz o impacto dos
    // extremos sem perder informacao.
    lf = lf.with_column(
        when(col("total_loss_usd").gt(lit(params.iqr_upper)))
            .then(lit(params.iqr_upper))
            .otherwise(col("total_loss_usd"))
            .alias("total_loss_usd")
    );

    // --- TRANSFORMACAO 6: Z-SCORE CLIPPING (company_revenue_usd) ---
    // Justificativa: Segunda tecnica de tratamento de outliers.
    // Valores com |z| >= 3 (aprendido no treino) sao cortados.
    // Combinar IQR + Z-score da mais robustez ao tratamento.
    for (c, lower, upper) in &params.outlier_bounds {
        lf = lf.with_column(
            when(col(c.as_str()).gt(lit(*upper))).then(lit(*upper))
                .when(col(c.as_str()).lt(lit(*lower))).then(lit(*lower))
                .otherwise(col(c.as_str())).alias(c.as_str())
        );
    }

    let novo_df = lf.collect()?;
    println!("   TRANSFORM concluido! Dataset: {}x{}\n", novo_df.height(), novo_df.width());
    Ok(novo_df)
}

pub fn executar() -> Result<(), Box<dyn Error>> {
    println!("[OURO] Iniciando processamento...\n");

    let df = LazyFrame::scan_parquet(
        "camada_prata/dataset_ml.parquet",
        ScanArgsParquet::default(),
    )?.collect()?;

    // Selecionar apenas as colunas uteis para o modelo
    // Nota: direct_loss_usd foi removida propositalmente.
    // Ela e um componente direto de total_loss_usd (data leakage).
    let colunas_para_manter = [
        "company_revenue_usd", "employee_count", "is_public_company",
        "industry_primary", "attack_vector_primary", "data_type",
        "confidence_tier", "quality_score", "total_loss_usd",
    ];
    let cols_existentes: Vec<&str> = colunas_para_manter.iter()
        .filter(|c| df.column(c).is_ok()).copied().collect();
    let df_ml = df.select(cols_existentes)?;

    // --- PADRAO FIT/TRANSFORM CORRETO ---
    // 1. DIVIDIR dados em treino (80%) e teste (20%) ANTES de fit
    // 2. FIT apenas no TREINO (aprende medianas, medias, desvios, limites)
    // 3. TRANSFORM no TREINO (aplica parametros do proprio treino)
    // 4. TRANSFORM no TESTE (aplica MESMOS parametros do treino)
    // 5. COMBINAR e salvar
    //
    // Isso e EQUIVALENTE ao que sklearn faz:
    //   scaler = StandardScaler()
    //   scaler.fit(X_train)        # FIT no treino
    //   X_train_scaled = scaler.transform(X_train)
    //   X_test_scaled = scaler.transform(X_test)  # MESMO scaler
    let total = df_ml.height();
    let split = (total as f64 * 0.8) as usize;

    let df_treino = df_ml.slice(0, split);
    let df_teste = df_ml.slice(split as i64, total - split);

    println!("   Split treino/teste: {} treino, {} teste (80/20)", split, total - split);

    // FIT apenas no TREINO
    let params = fit(&df_treino)?;

    // TRANSFORM em ambos (com parametros do TREINO)
    let df_treino_t = transform(&df_treino, &params)?;
    let df_teste_t = transform(&df_teste, &params)?;

    // Combinar de volta (vertical stack)
    let mut df_ouro = df_treino_t.vstack(&df_teste_t)?;

    fs::create_dir_all("camada_ouro")?;
    let mut arquivo = File::create("camada_ouro/dataset_ml_ready.parquet")?;
    ParquetWriter::new(&mut arquivo).finish(&mut df_ouro)?;
    println!("   Dataset Ouro salvo: camada_ouro/dataset_ml_ready.parquet");

    // Tabela de transformacoes documentada (requisito do texto.md)
    println!("\nTABELA DE TRANSFORMACOES - CAMADA OURO:");
    println!("   ------------------------------------------------------------------------------------");
    println!("   Transformacao        | Tecnica              | Colunas");
    println!("   ------------------------------------------------------------------------------------");
    println!("   Label Encoding       | Cast str -> int32    | confidence_tier");
    println!("   One-Hot Encoding     | Dummy encoding       | industry_primary ({} cats), attack_vector_primary, data_type",
        params.onehot_specs.iter().filter(|(c,_)| c == "industry_primary").count());
    println!("   Missing (numerico)   | Mediana (robusta)    | company_revenue_usd, employee_count, quality_score");
    println!("   Missing (categorico) | 'Desconhecido'       | (feito na Prata)");
    println!("   Standard Scaling     | Z-score (treino)     | company_revenue_usd, employee_count");
    println!("   Outlier (IQR)        | Clipping 1.5x IQR    | total_loss_usd");
    println!("   Outlier (Z-score)    | Clipping |z|<3       | company_revenue_usd");
    println!("   ------------------------------------------------------------------------------------");
    println!("   Observacao: FIT usado dados de TREINO, TRANSFORM aplicado em ambos.");
    println!("   Nenhuma informacao do teste influenciou o aprendizado dos parametros.\n");

    println!("   [OURO] Concluida!\n");
    Ok(())
}

#[allow(dead_code)]
pub fn carregar_para_ml() -> Result<(DataFrame, DataFrame), Box<dyn Error>> {
    let df_prata = LazyFrame::scan_parquet(
        "camada_prata/dataset_ml.parquet", ScanArgsParquet::default(),
    )?.collect()?;
    let df_ouro = LazyFrame::scan_parquet(
        "camada_ouro/dataset_ml_ready.parquet", ScanArgsParquet::default(),
    )?.collect()?;
    Ok((df_prata, df_ouro))
}

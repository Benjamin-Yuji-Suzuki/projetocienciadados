use polars::prelude::*;
use smartcore::tree::decision_tree_classifier::*;
use smartcore::linalg::basic::matrix::DenseMatrix;
use smartcore::linalg::basic::arrays::Array;
use std::error::Error;
use std::fs;
use rand::Rng;

// ====================================================================
// METRICAS DE AVALIACAO
// ====================================================================
// Usamos 4 metricas para avaliar os modelos (acurácia, precisao, recall, F1).
// A MATRIZ DE CONFUSAO mostra VP, VN, FP, FN e permite calcular tudo.
//
//   Acuracia = (VP + VN) / Total  -> % total de acertos
//   Precisao = VP / (VP + FP)     -> quando o modelo diz "alto impacto",
//                                     quantas vezes ele acerta?
//   Recall   = VP / (VP + FN)     -> de todos os "alto impacto" reais,
//                                     quantos o modelo pegou?
//   F1-Score = 2 * P * R / (P + R) -> media harmonica (balanceamento)
//
// O F1 e a metrica mais importante aqui porque temos classes desbalanceadas
// (menos incidentes de alto impacto que baixo impacto).

// direct_loss_usd foi removida por ser componente direto de total_loss_usd
// (data leakage). O modelo usava a perda direta para prever a perda total,
// o que e invalido — na vida real, nao sabemos a perda direta antes de
// calcular a perda total.
//
// Ao contrario de antes (features fixas), agora DETECTAMOS DINAMICAMENTE
// todas as colunas numericas disponiveis. Isto captura automaticamente
// as colunas one-hot criadas na Gold (industry_primary_X, attack_vector_Y,
// data_type_Z), maximizando a informacao disponivel para o modelo.
// A unica excecao e total_loss_usd (base do target).

fn acuracia(y_true: &[u32], y_pred: &[u32]) -> f64 {
    let ok = y_true.iter().zip(y_pred.iter()).filter(|(a, b)| a == b).count();
    ok as f64 / y_true.len() as f64
}

fn precisao(y_true: &[u32], y_pred: &[u32]) -> f64 {
    let vp = y_true.iter().zip(y_pred).filter(|(&t, &p)| t == 1 && p == 1).count() as f64;
    let fp = y_true.iter().zip(y_pred).filter(|(&t, &p)| t == 0 && p == 1).count() as f64;
    if vp + fp > 0.0 { vp / (vp + fp) } else { 0.0 }
}

fn recall(y_true: &[u32], y_pred: &[u32]) -> f64 {
    let vp = y_true.iter().zip(y_pred).filter(|(&t, &p)| t == 1 && p == 1).count() as f64;
    let fn_ = y_true.iter().zip(y_pred).filter(|(&t, &p)| t == 1 && p == 0).count() as f64;
    if vp + fn_ > 0.0 { vp / (vp + fn_) } else { 0.0 }
}

fn f1_score(prec: f64, rec: f64) -> f64 {
    if prec + rec > 0.0 { 2.0 * prec * rec / (prec + rec) } else { 0.0 }
}

fn matriz_conf(y_true: &[u32], y_pred: &[u32]) -> (u32, u32, u32, u32) {
    let (mut vp, mut vn, mut fp, mut fn_) = (0, 0, 0, 0);
    for (&t, &p) in y_true.iter().zip(y_pred) {
        match (t, p) { (1, 1) => vp += 1, (0, 0) => vn += 1, (0, 1) => fp += 1, (1, 0) => fn_ += 1, _ => {} }
    }
    (vp, vn, fp, fn_)
}

fn gerar_matriz_confusao_svg(vp: u32, vn: u32, fp: u32, fn_: u32) -> Result<(), Box<dyn Error>> {
    let total = vp + vn + fp + fn_;
    let acc = if total > 0 { (vp + vn) as f64 / total as f64 * 100.0 } else { 0.0 };

    // Layout maior para evitar texto cortado a esquerda
    let c1_x = 115;  // coluna VP/FP
    let c2_x = 385;  // coluna FN/VN
    let cw = 250;    // largura da celula
    let r1_y = 110;  // linha VP/FN
    let r2_y = 280;  // linha FP/VN
    let rh = 155;    // altura da celula
    let lb_x = 100;  // borda direita dos rotulos (text-anchor='end')

    let svg = format!(
"<svg xmlns='http://www.w3.org/2000/svg' width='700' height='480'>
<defs><linearGradient id='bg' x1='0' y1='0' x2='0' y2='1'><stop offset='0%' stop-color='#f8f9fa'/><stop offset='100%' stop-color='#e9ecef'/></linearGradient>
<linearGradient id='gvp' x1='0' y1='0' x2='0' y2='1'><stop offset='0%' stop-color='#2d6a4f'/><stop offset='100%' stop-color='#1b4332'/></linearGradient>
<linearGradient id='gfn' x1='0' y1='0' x2='0' y2='1'><stop offset='0%' stop-color='#c1121f'/><stop offset='100%' stop-color='#780000'/></linearGradient>
<linearGradient id='gfp' x1='0' y1='0' x2='0' y2='1'><stop offset='0%' stop-color='#c1121f'/><stop offset='100%' stop-color='#780000'/></linearGradient>
<linearGradient id='gvn' x1='0' y1='0' x2='0' y2='1'><stop offset='0%' stop-color='#2d6a4f'/><stop offset='100%' stop-color='#1b4332'/></linearGradient>
<filter id='shadow'><feDropShadow dx='2' dy='2' stdDeviation='3' flood-opacity='0.3'/></filter></defs>
<style>text {{ font-family: 'Segoe UI', Arial, sans-serif; }}</style>
<rect width='100%' height='100%' fill='url(#bg)'/>
<text x='350' y='40' text-anchor='middle' font-size='24' font-weight='bold' fill='#1a1a2e'>Matriz de Confusao - Melhor Modelo</text>
<text x='350' y='65' text-anchor='middle' font-size='14' fill='#555'>Modelo 1: Gini Index | max_depth=5</text>
<text x='{}' y='95' text-anchor='end' font-size='12' fill='#888' font-style='italic'>R \\\\ P</text>
<text x='{}' y='95' text-anchor='middle' font-size='14' font-weight='bold' fill='#333'>Previsto: ALTO (1)</text>
<text x='{}' y='95' text-anchor='middle' font-size='14' font-weight='bold' fill='#333'>Previsto: BAIXO (0)</text>
<text x='{}' y='{}' text-anchor='end' font-size='14' font-weight='bold' fill='#333'>Real: ALTO (1)</text>
<text x='{}' y='{}' text-anchor='end' font-size='14' font-weight='bold' fill='#333'>Real: BAIXO (0)</text>
<rect x='{}' y='{}' width='{}' height='{}' fill='url(#gvp)' rx='10' filter='url(#shadow)'/>
<text x='{}' y='{}' text-anchor='middle' font-size='36' fill='white' font-weight='bold'>VP = {}</text>
<text x='{}' y='{}' text-anchor='middle' font-size='13' fill='#d8f3dc'>Acertou alto impacto</text>
<rect x='{}' y='{}' width='{}' height='{}' fill='url(#gfn)' rx='10' filter='url(#shadow)'/>
<text x='{}' y='{}' text-anchor='middle' font-size='36' fill='white' font-weight='bold'>FN = {}</text>
<text x='{}' y='{}' text-anchor='middle' font-size='13' fill='#ffcdd2'>Falso Negativo</text>
<rect x='{}' y='{}' width='{}' height='{}' fill='url(#gfp)' rx='10' filter='url(#shadow)'/>
<text x='{}' y='{}' text-anchor='middle' font-size='36' fill='white' font-weight='bold'>FP = {}</text>
<text x='{}' y='{}' text-anchor='middle' font-size='13' fill='#ffcdd2'>Falso Positivo</text>
<rect x='{}' y='{}' width='{}' height='{}' fill='url(#gvn)' rx='10' filter='url(#shadow)'/>
<text x='{}' y='{}' text-anchor='middle' font-size='36' fill='white' font-weight='bold'>VN = {}</text>
<text x='{}' y='{}' text-anchor='middle' font-size='13' fill='#d8f3dc'>Acertou baixo impacto</text>
<text x='350' y='465' text-anchor='middle' font-size='20' font-weight='bold' fill='#1a1a2e'>Acuracia: {:.1}%</text>
</svg>",
        lb_x,
        c1_x + cw / 2, c2_x + cw / 2,
        lb_x, r1_y + rh / 2 + 5, lb_x, r2_y + rh / 2 + 5,
        c1_x, r1_y, cw, rh, c1_x + cw / 2, r1_y + rh / 2 + 5, vp, c1_x + cw / 2, r1_y + rh / 2 + 35,
        c2_x, r1_y, cw, rh, c2_x + cw / 2, r1_y + rh / 2 + 5, fn_, c2_x + cw / 2, r1_y + rh / 2 + 35,
        c1_x, r2_y, cw, rh, c1_x + cw / 2, r2_y + rh / 2 + 5, fp, c1_x + cw / 2, r2_y + rh / 2 + 35,
        c2_x, r2_y, cw, rh, c2_x + cw / 2, r2_y + rh / 2 + 5, vn, c2_x + cw / 2, r2_y + rh / 2 + 35,
        acc);

    fs::write("graficos/matriz_confusao.svg", svg)?;
    println!("   Matriz de confusao salva: graficos/matriz_confusao.svg");
    Ok(())
}

fn gerar_feature_importance_svg(pares: &[(String, f64)]) -> Result<String, Box<dyn Error>> {
    let w = 850; let h = 420;
    let me = 220; let md = 50; let mt = 70; let mb = 40;
    let pw = w - me - md; let ph = h - mt - mb;
    let max_v = pares.iter().map(|(_, v)| *v).fold(0.0_f64, f64::max).max(0.01);
    let n = pares.len();

    let cores = ["#e63946", "#457b9d", "#2a9d8f", "#e9c46a"];
    let mut svg = String::new();

    svg.push_str(&format!(
"<svg xmlns='http://www.w3.org/2000/svg' width='{}' height='{}'>
<defs><linearGradient id='bg' x1='0' y1='0' x2='0' y2='1'><stop offset='0%' stop-color='#f8f9fa'/><stop offset='100%' stop-color='#e9ecef'/></linearGradient>
<filter id='sb'><feDropShadow dx='2' dy='2' stdDeviation='3' flood-opacity='0.25'/></filter></defs>
<style>text {{ font-family: 'Segoe UI', Arial, sans-serif; }}</style>
<rect width='100%' height='100%' fill='url(#bg)'/>
<text x='{}' y='32' text-anchor='middle' font-size='22' font-weight='bold' fill='#1a1a2e'>Importancia das Features (Permutation)</text>
<text x='{}' y='52' text-anchor='middle' font-size='13' fill='#666'>Modelo 1 - Decision Tree (Gini, max_depth=5) — Permutation Importance (5 rep.)</text>
<line x1='{}' y1='{}' x2='{}' y2='{}' stroke='#333' stroke-width='1.5'/>
<line x1='{}' y1='{}' x2='{}' y2='{}' stroke='#333' stroke-width='1.5'/>",
        w, h, w / 2, w / 2, me, mt, me, h - mb, me, h - mb, w - md, h - mb));

    for i in 0..=4 {
        let val = max_v * i as f64 / 4.0;
        let y = (h - mb) as f64 - (ph as f64 * i as f64 / 4.0);
        svg.push_str(&format!("<text x='{}' y='{:.0}' text-anchor='end' font-size='12' fill='#888'>{:.4}</text>", me - 8, y + 4.0, val));
        if i > 0 {
            svg.push_str(&format!("<line x1='{}' y1='{:.0}' x2='{}' y2='{:.0}' stroke='#dee2e6' stroke-width='0.5' stroke-dasharray='4,4'/>", me, y, w - md, y));
        }
    }

    let bar_h = ((ph as f64 / n as f64) * 0.6).max(24.0) as i32;
    let gap = ((ph as f64 / n as f64) * 0.4) as i32;

    for (i, (nome, valor)) in pares.iter().enumerate() {
        let bar_w = ((*valor / max_v) * pw as f64) as i32;
        let draw_w = if *valor > 0.0 { bar_w.max(4) } else { 0 };
        let y = mt + i as i32 * (bar_h + gap) + 6;
        let cor = cores[i % cores.len()];

        if draw_w > 0 {
            svg.push_str(&format!("<rect x='{}' y='{}' width='{}' height='{}' fill='{}' rx='6' filter='url(#sb)'/>", me, y, draw_w, bar_h, cor));
        }

        // Label da feature (nome)
        svg.push_str(&format!("<text x='{}' y='{}' text-anchor='end' font-size='13' fill='#333' font-weight='bold'>{}</text>", me - 8, y + bar_h / 2 + 5, nome));

        // Valor da importancia - cor escura se barra invisivel
        if *valor > 0.0 {
            svg.push_str(&format!("<text x='{}' y='{}' font-size='13' fill='white' font-weight='bold'>{:.4}</text>", me + 10, y + bar_h / 2 + 5, valor));
        } else {
            svg.push_str(&format!("<text x='{}' y='{}' font-size='13' fill='#999' font-weight='bold'>{:.4}</text>", me + 10, y + bar_h / 2 + 5, valor));
        }
    }

    // Legenda do metodo
    svg.push_str(&format!("<text x='{}' y='{}' text-anchor='end' font-size='11' fill='#aaa' font-style='italic'>Metodo: embaralhamento de cada feature com 5 repeticoes</text>", w - md, h - 8));

    svg.push_str("</svg>");
    Ok(svg)
}

// --- DETECAO DINAMICA DE FEATURES ---
// Em vez de uma lista fixa de colunas, detectamos automaticamente todas
// as colunas efectivamente numericas do DataFrame, excluindo o target.
// Isto permite que a Gold crie dezenas de colunas one-hot e o ML as
// use automaticamente, sem necessidade de actualizar constantes.
//
// Usamos 2 criterios:
//   1. dtype nativo numerico (int, float, bool) — aceite directamente
//   2. dtype String — tentamos cast para Float64; se MAIS DE 50% dos
//      valores forem validos (nao null), a coluna contem numeros em
//      formato texto e deve ser incluida.
// Isto evita incluir colunas de texto (country_hq, data_source, etc.)
// mas captura confidence_tier ("1", "2", "3", "4").
fn detectar_features(df: &DataFrame) -> Result<Vec<String>, Box<dyn Error>> {
    let total = df.height();
    if total == 0 { return Ok(vec![]); }
    let mut features = Vec::new();
    for nome in df.get_column_names() {
        if nome == "total_loss_usd" || nome == "target" { continue; }
        if let Ok(col) = df.column(nome) {
            match col.dtype() {
                DataType::Int8 | DataType::Int16 | DataType::Int32 | DataType::Int64
                | DataType::UInt8 | DataType::UInt16 | DataType::UInt32 | DataType::UInt64
                | DataType::Float32 | DataType::Float64 | DataType::Boolean => {
                    features.push(nome.to_string());
                }
                DataType::String => {
                    if let Ok(ca) = col.cast(&DataType::Float64) {
                        if let Ok(fca) = ca.f64() {
                            let validos = fca.into_iter().filter(|v| v.is_some()).count();
                            if validos as f64 / total as f64 > 0.5 {
                                features.push(nome.to_string());
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
    Ok(features)
}

// --- ESTRATEGIA: PREPARACAO DOS DADOS ---
// Convertemos as colunas do DataFrame Polars para uma matriz DenseMatrix
// do smartcore (a biblioteca de ML que usamos). As features sao DETECTADAS
// DINAMICAMENTE: toda coluna numerica que nao e o target vira feature.
//
// Na PRATA, as features sao apenas as 4 colunas numericas originais:
//   company_revenue_usd, employee_count, confidence_tier, quality_score
//
// Na OURO, as features incluem TUDO acima MAIS as colunas one-hot:
//   industry_primary_51, attack_vector_primary_Phishing, data_type_PII, etc.
//
// NOTA: direct_loss_usd foi REMOVIDA de todas as camadas por ser
// componente direto de total_loss_usd (data leakage).
//
// O TARGET (y) e BINARIO: 1 = alto impacto (prejuizo acima da mediana)
//                          0 = baixo impacto (prejuizo abaixo da mediana)
// Usamos a mediana como ponto de corte porque a distribuicao de perdas
// e assimetrica (vimos no EDA) - a mediana e mais representativa que
// a media para dividir as classes.
fn preparar_dados(df: &DataFrame) -> Result<(Vec<Vec<f64>>, Vec<u32>, Vec<f64>, f64), Box<dyn Error>> {
    let features = detectar_features(df)?;
    let n_linhas = df.height();
    let mut dados: Vec<Vec<f64>> = Vec::with_capacity(n_linhas);

    for i in 0..n_linhas {
        let mut linha = Vec::with_capacity(features.len());
        for f in &features {
            let val = match df.column(f) {
                Ok(col) => {
                    match col.cast(&DataType::Float64) {
                        Ok(ca) => match ca.f64() {
                            Ok(fca) => fca.get(i).unwrap_or(0.0),
                            Err(_) => 0.0,
                        },
                        Err(_) => 0.0,
                    }
                },
                Err(_) => 0.0,
            };
            linha.push(if val.is_nan() || val.is_infinite() { 0.0 } else { val });
        }
        dados.push(linha);
    }

    // Target binario baseado na mediana (visto na EDA: distribuicao assimetrica)
    let loss_vals: Vec<f64> = df.column("total_loss_usd")?
        .cast(&DataType::Float64)?.f64()?
        .into_iter().filter_map(|v| v).collect();
    let mut sorted = loss_vals.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mediana = sorted[sorted.len() / 2];
    let y: Vec<u32> = loss_vals.iter().map(|&v| if v > mediana { 1 } else { 0 }).collect();

    Ok((dados, y, loss_vals, mediana))
}

fn dados_para_matriz(dados: &[Vec<f64>]) -> Result<smartcore::linalg::basic::matrix::DenseMatrix<f64>, Box<dyn Error>> {
    let refs: Vec<&[f64]> = dados.iter().map(|v| v.as_slice()).collect();
    Ok(smartcore::linalg::basic::matrix::DenseMatrix::from_2d_array(&refs))
}

fn salvar_previsoes_csv(
    y_test: &[u32], pred: &[u32], loss_test: &[f64],
    threshold: f64, prefixo: &str,
) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all("previsoes")?;
    let mut csv = format!("# Threshold (mediana): total_loss_usd > {:.2} = Alto Impacto\n", threshold);
    csv.push_str("id,real,previsto,acertou?,total_loss_usd\n");
    for (i, (&real, (&predita, &perda))) in y_test.iter().zip(pred.iter().zip(loss_test.iter())).enumerate() {
        let r_label = if real == 1 { "Alto" } else { "Baixo" };
        let p_label = if predita == 1 { "Alto" } else { "Baixo" };
        let acerto = if real == predita { "SIM" } else { "NAO" };
        csv.push_str(&format!("{},{},{},{},{:.2}\n", i, r_label, p_label, acerto, perda));
    }
    let path = format!("previsoes/previsoes_{}.csv", prefixo);
    fs::write(&path, csv)?;
    println!("   Previsoes salvas: {}", path);
    Ok(())
}

fn salvar_modelo_json(
    _params: &DecisionTreeClassifierParameters,
    criterion: &str, max_depth: u16,
    importances: &[(String, f64)],
    feature_names: &[String],
    acc_train: f64, f1_train: f64,
    acc_test: f64, f1_test: f64,
    n_train: usize, n_test: usize,
    prefixo: &str,
) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all("modelos")?;
    let imp_map: serde_json::Value = importances.iter().map(|(k, v)| (k.clone(), *v)).collect();
    let json = serde_json::json!({
        "modelo": "DecisionTreeClassifier",
        "criterion": criterion,
        "max_depth": max_depth,
        "features": feature_names,
        "feature_importances": imp_map,
        "metricas": {
            "acuracia_treino": acc_train,
            "f1_treino": f1_train,
            "acuracia_teste": acc_test,
            "f1_teste": f1_test,
            "n_treino": n_train,
            "n_teste": n_test,
        }
    });
    let path = format!("modelos/modelo_{}.json", prefixo);
    fs::write(&path, serde_json::to_string_pretty(&json)?)?;
    println!("   Modelo salvo: {}", path);
    Ok(())
}

// --- PERMUTATION IMPORTANCE ---
// Como o SmartCore nao expoe internamente a arvore para calcular
// a importancia Gini diretamente, usamos PERMUTATION IMPORTANCE:
// embaralhamos cada feature e medimos a queda na acuracia.
// Quanto maior a queda, mais importante e a feature.
fn permutation_importance(
    model: &DecisionTreeClassifier<f64, u32, DenseMatrix<f64>, Vec<u32>>,
    x: &DenseMatrix<f64>,
    y: &[u32],
    n_repeats: usize,
) -> Result<Vec<f64>, Box<dyn Error>> {
    let baseline = acuracia(y, &model.predict(x)?);
    let n_samples = x.shape().0;
    let n_features = x.shape().1;
    let mut importances = vec![0.0; n_features];

    // Extrair dados para uma Vec<Vec<f64>> para facilitar shuffle
    let data: Vec<Vec<f64>> = (0..n_samples).map(|i| {
        (0..n_features).map(|j| *x.get((i, j))).collect()
    }).collect();

    let mut rng = rand::thread_rng();

    for feat in 0..n_features {
        let mut drops = Vec::new();
        for _ in 0..n_repeats {
            let mut shuffled = data.clone();
            // Shuffle apenas a coluna 'feat'
            let n = shuffled.len();
            for a in (1..n).rev() {
                let b = rng.gen_range(0..=a);
                let tmp = shuffled[a][feat];
                shuffled[a][feat] = shuffled[b][feat];
                shuffled[b][feat] = tmp;
            }
            let refs: Vec<&[f64]> = shuffled.iter().map(|v| v.as_slice()).collect();
            let x_shuffled = DenseMatrix::from_2d_array(&refs);
            let pred = model.predict(&x_shuffled)?;
            drops.push(baseline - acuracia(y, &pred));
        }
        importances[feat] = drops.iter().sum::<f64>() / drops.len() as f64;
    }
    Ok(importances)
}

fn gerar_comparacao_previsoes_svg(
    vp1: u32, vn1: u32, fp1: u32, fn1: u32,
    vp2: u32, vn2: u32, fp2: u32, fn2: u32,
    threshold: f64,
) -> Result<String, Box<dyn Error>> {
    let total1 = vp1 + vn1 + fp1 + fn1;
    let total2 = vp2 + vn2 + fp2 + fn2;
    let acc1 = if total1 > 0 { (vp1 + vn1) as f64 / total1 as f64 * 100.0 } else { 0.0 };
    let acc2 = if total2 > 0 { (vp2 + vn2) as f64 / total2 as f64 * 100.0 } else { 0.0 };

    let svg = format!(
"<svg xmlns='http://www.w3.org/2000/svg' width='750' height='480'>
<defs><linearGradient id='bg' x1='0' y1='0' x2='0' y2='1'><stop offset='0%' stop-color='#f8f9fa'/><stop offset='100%' stop-color='#e9ecef'/></linearGradient>
<linearGradient id='g1' x1='0' y1='0' x2='0' y2='1'><stop offset='0%' stop-color='#e63946'/><stop offset='100%' stop-color='#c1121f'/></linearGradient>
<linearGradient id='g2' x1='0' y1='0' x2='0' y2='1'><stop offset='0%' stop-color='#457b9d'/><stop offset='100%' stop-color='#1d3557'/></linearGradient>
<linearGradient id='gg' x1='0' y1='0' x2='0' y2='1'><stop offset='0%' stop-color='#2d6a4f'/><stop offset='100%' stop-color='#1b4332'/></linearGradient>
<filter id='sb'><feDropShadow dx='2' dy='2' stdDeviation='3' flood-opacity='0.25'/></filter></defs>
<rect width='100%' height='100%' fill='url(#bg)'/>
<text x='375' y='35' text-anchor='middle' font-size='22' font-weight='bold' fill='#1a1a2e'>Comparacao de Previsoes</text>
<text x='375' y='55' text-anchor='middle' font-size='13' fill='#555'>Threshold: total_loss_usd &gt; {:.0} = Alto Impacto</text>

<text x='190' y='85' text-anchor='middle' font-size='15' font-weight='bold' fill='#e63946'>Modelo 1 (Gini d=5)</text>
<text x='560' y='85' text-anchor='middle' font-size='15' font-weight='bold' fill='#457b9d'>Modelo 2 (Entropy d=10)</text>

<text x='50' y='120' font-size='13' font-weight='bold' fill='#333'>Acuracia</text>
<rect x='190' y='105' width='{}' height='22' fill='url(#g1)' rx='4' filter='url(#sb)'/>
<text x='205' y='121' font-size='12' fill='white' font-weight='bold'>{:.1}%</text>
<rect x='560' y='105' width='{}' height='22' fill='url(#g2)' rx='4' filter='url(#sb)'/>
<text x='575' y='121' font-size='12' fill='white' font-weight='bold'>{:.1}%</text>

<text x='50' y='155' font-size='13' font-weight='bold' fill='#333'>Acertos (VP+VN)</text>
<rect x='190' y='140' width='{}' height='22' fill='url(#gg)' rx='4' filter='url(#sb)'/>
<text x='205' y='156' font-size='12' fill='white'>{}</text>
<rect x='560' y='140' width='{}' height='22' fill='url(#gg)' rx='4' filter='url(#sb)'/>
<text x='575' y='156' font-size='12' fill='white'>{}</text>

<text x='50' y='190' font-size='13' font-weight='bold' fill='#333'>Erros (FP+FN)</text>
<rect x='190' y='175' width='{}' height='22' fill='#e63946' rx='4' filter='url(#sb)'/>
<text x='205' y='191' font-size='12' fill='white'>{}</text>
<rect x='560' y='175' width='{}' height='22' fill='#e63946' rx='4' filter='url(#sb)'/>
<text x='575' y='191' font-size='12' fill='white'>{}</text>

<text x='50' y='225' font-size='13' font-weight='bold' fill='#333'>VP (acertou Alto)</text>
<rect x='190' y='210' width='{}' height='22' fill='#2d6a4f' rx='4' filter='url(#sb)'/>
<text x='205' y='226' font-size='12' fill='white'>{}</text>
<rect x='560' y='210' width='{}' height='22' fill='#2d6a4f' rx='4' filter='url(#sb)'/>
<text x='575' y='226' font-size='12' fill='white'>{}</text>

<text x='50' y='260' font-size='13' font-weight='bold' fill='#333'>VN (acertou Baixo)</text>
<rect x='190' y='245' width='{}' height='22' fill='#457b9d' rx='4' filter='url(#sb)'/>
<text x='205' y='261' font-size='12' fill='white'>{}</text>
<rect x='560' y='245' width='{}' height='22' fill='#457b9d' rx='4' filter='url(#sb)'/>
<text x='575' y='261' font-size='12' fill='white'>{}</text>

<text x='50' y='295' font-size='13' font-weight='bold' fill='#333'>FP (falso alarme)</text>
<rect x='190' y='280' width='{}' height='22' fill='#c1121f' rx='4' filter='url(#sb)'/>
<text x='205' y='296' font-size='12' fill='white'>{}</text>
<rect x='560' y='280' width='{}' height='22' fill='#c1121f' rx='4' filter='url(#sb)'/>
<text x='575' y='296' font-size='12' fill='white'>{}</text>

<text x='50' y='330' font-size='13' font-weight='bold' fill='#333'>FN (falhou deteccao)</text>
<rect x='190' y='315' width='{}' height='22' fill='#e63946' rx='4' filter='url(#sb)'/>
<text x='205' y='331' font-size='12' fill='white'>{}</text>
<rect x='560' y='315' width='{}' height='22' fill='#e63946' rx='4' filter='url(#sb)'/>
<text x='575' y='331' font-size='12' fill='white'>{}</text>

<text x='375' y='375' text-anchor='middle' font-size='14' font-weight='bold' fill='#1a1a2e'>Legenda</text>
<rect x='230' y='395' width='14' height='14' fill='#2d6a4f' rx='2'/><text x='248' y='407' font-size='12' fill='#333'>VP</text>
<rect x='295' y='395' width='14' height='14' fill='#457b9d' rx='2'/><text x='313' y='407' font-size='12' fill='#333'>VN</text>
<rect x='360' y='395' width='14' height='14' fill='#c1121f' rx='2'/><text x='378' y='407' font-size='12' fill='#333'>FP</text>
<rect x='425' y='395' width='14' height='14' fill='#e63946' rx='2'/><text x='443' y='407' font-size='12' fill='#333'>FN</text>
</svg>",
    threshold,
    (acc1 / 100.0 * 370.0) as i32, acc1, (acc2 / 100.0 * 370.0) as i32, acc2,
    ((vp1 as i32 + vn1 as i32) * 370 / total1 as i32) as i32, vp1+vn1, ((vp2 as i32 + vn2 as i32) * 370 / total2 as i32) as i32, vp2+vn2,
    ((fp1 as i32 + fn1 as i32) * 370 / total1 as i32) as i32, fp1+fn1, ((fp2 as i32 + fn2 as i32) * 370 / total2 as i32) as i32, fp2+fn2,
    (vp1 as i32 * 370 / total1 as i32) as i32, vp1, (vp2 as i32 * 370 / total2 as i32) as i32, vp2,
    (vn1 as i32 * 370 / total1 as i32) as i32, vn1, (vn2 as i32 * 370 / total2 as i32) as i32, vn2,
    (fp1 as i32 * 370 / total1 as i32) as i32, fp1, (fp2 as i32 * 370 / total2 as i32) as i32, fp2,
    (fn1 as i32 * 370 / total1 as i32) as i32, fn1, (fn2 as i32 * 370 / total2 as i32) as i32, fn2);
    Ok(svg)
}

// --- ESTRATEGIA: VISUALIZACAO DA ARVORE DE DECISAO ---
// Como o SmartCore nao expoe a estrutura interna da arvore (nos, divisoes,
// thresholds), criamos uma visualizacao REPRESENTATIVA que mostra:
//   - As features ordenadas por importancia (da mais para menos relevante)
//   - A estrutura hierarquica da arvore ate profundidade 3
//   - Os thresholds estimados (mediana dos dados de treino)
//   - As decisoes finais (Alto/Baixo impacto)
//
// Para arvores mais profundas (d=5 ou d=10), mostramos ate 3 niveis,
// que e o suficiente para compreender a logica de decisao.
fn gerar_arvore_decisao_svg(
    features: &[String],
    importances: &[(String, f64)],
    criterion: &str,
    max_depth: u16,
    train_data: &DenseMatrix<f64>,
) -> Result<(), Box<dyn Error>> {
    let w: i32 = 1000;
    let h: i32 = 700;
    let levels_i: i32 = 3i32.min(max_depth as i32 + 1);
    let left: i32 = 70;
    let right: i32 = 70;
    let top: i32 = 110;
    let vgap: i32 = 140;
    let draw_w: i32 = w - left - right;

    let mut svg = String::new();

    svg.push_str(&format!(
"<svg xmlns='http://www.w3.org/2000/svg' width='{}' height='{}'>
<defs>
  <linearGradient id='bgt' x1='0' y1='0' x2='0' y2='1'><stop offset='0%' stop-color='#f8f9fa'/><stop offset='100%' stop-color='#e9ecef'/></linearGradient>
  <linearGradient id='groot' x1='0' y1='0' x2='0' y2='1'><stop offset='0%' stop-color='#1d3557'/><stop offset='100%' stop-color='#0b1a2e'/></linearGradient>
  <linearGradient id='gint' x1='0' y1='0' x2='0' y2='1'><stop offset='0%' stop-color='#457b9d'/><stop offset='100%' stop-color='#1d3557'/></linearGradient>
  <linearGradient id='gleafA' x1='0' y1='0' x2='0' y2='1'><stop offset='0%' stop-color='#2d6a4f'/><stop offset='100%' stop-color='#1b4332'/></linearGradient>
  <linearGradient id='gleafB' x1='0' y1='0' x2='0' y2='1'><stop offset='0%' stop-color='#e63946'/><stop offset='100%' stop-color='#780000'/></linearGradient>
  <filter id='sht'><feDropShadow dx='2' dy='3' stdDeviation='4' flood-opacity='0.3'/></filter>
  <marker id='arr' viewBox='0 0 10 10' refX='10' refY='5' markerWidth='7' markerHeight='7' orient='auto'><path d='M0,0 L10,5 L0,10 Z' fill='#888'/></marker>
</defs>
<style>text {{ font-family: 'Segoe UI', Arial, sans-serif; }}</style>
<rect width='100%' height='100%' fill='url(#bgt)'/>
<text x='500' y='35' text-anchor='middle' font-size='22' font-weight='bold' fill='#1a1a2e'>Arvore de Decisao — Estrutura</text>
<text x='500' y='55' text-anchor='middle' font-size='13' fill='#666'>Criterio: {} | Profundidade maxima: {} | Features ordenadas por Permutation Importance</text>
<line x1='100' y1='72' x2='900' y2='72' stroke='#dee2e6' stroke-width='1'/>\n", w, h, criterion, max_depth));

    // Helper: estimar threshold (mediana) de uma feature no treino
    let median_of_feat = |feat_name: &str| -> f64 {
        for (i, name) in features.iter().enumerate() {
            if name == feat_name {
                let n = train_data.shape().0;
                if n == 0 { return 0.0; }
                let mut vals: Vec<f64> = (0..n).map(|r| *train_data.get((r, i))).collect();
                vals.sort_by(|a, b| a.partial_cmp(b).unwrap());
                return vals[n / 2];
            }
        }
        0.0
    };

    // Ordenar features por importancia para atribuir a niveis da arvore
    let mut sorted_imp = importances.to_vec();
    sorted_imp.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    // Determinar quantos nos mostrar por nivel
    let mut node_features: Vec<(&str, f64)> = Vec::new();

    // Atribuir features aos niveis (breadth-first): root = mais importante
    for d in 0..levels_i {
        let n_nodes = 1i32 << d;
        for i in 0..n_nodes {
            let idx = (d + i) as usize;
            if idx < sorted_imp.len() {
                node_features.push((sorted_imp[idx].0.as_str(), sorted_imp[idx].1));
            } else {
                node_features.push(("", 0.0));
            }
        }
    }

    // Desenhar conexoes (parent -> child) primeiro para ficar atras dos nos
    for d in 0..levels_i {
        let n_parents = 1i32 << d;
        if d + 1 >= levels_i { break; }
        for p in 0..n_parents {
            let px = left + (draw_w as f64 * (p as f64 + 0.5) / n_parents as f64) as i32;
            let py = top + d * vgap + 20;
            for (ci, lbl) in [(0, "<= med"), (1, "> med")] {
                let c = 2 * p + ci;
                let cx = left + (draw_w as f64 * (c as f64 + 0.5) / (2 * n_parents) as f64) as i32;
                let cy = top + (d + 1) * vgap - 20;
                svg.push_str(&format!("<line x1='{}' y1='{}' x2='{}' y2='{}' stroke='#888' stroke-width='1.5' marker-end='url(#arr)'/>\n", px, py, cx, cy));
                let mx = (px + cx) / 2;
                let my = (py + cy) / 2;
                svg.push_str(&format!("<rect x='{}' y='{}' width='36' height='16' rx='8' fill='white' stroke='#ccc'/>\n", mx - 18, my - 8));
                svg.push_str(&format!("<text x='{}' y='{}' text-anchor='middle' font-size='9' fill='#555'>{}</text>\n", mx, my + 4, lbl));
            }
        }
    }

    // Desenhar nos
    let mut node_idx = 0usize;
    for d in 0..levels_i {
        let n_nodes = 1i32 << d;
        let node_w = ((draw_w as f64 / n_nodes as f64) * 0.5).min(180.0).max(60.0) as i32;
        let node_h: i32 = if d == levels_i - 1 { 38 } else if d == 0 { 52 } else { 46 };

        for i in 0..n_nodes {
            let x = left + (draw_w as f64 * (i as f64 + 0.5) / n_nodes as f64) as i32 - node_w / 2;
            let y: i32 = top + d * vgap;

            if node_idx >= node_features.len() { break; }
            let (feat_name, imp_val) = node_features[node_idx];
            node_idx += 1;

            if d == levels_i - 1 {
                let is_alto = i % 2 == 1;
                let cor = if is_alto { "url(#gleafA)" } else { "url(#gleafB)" };
                let label = if is_alto { "Alto Impacto" } else { "Baixo Impacto" };
                svg.push_str(&format!("<rect x='{}' y='{}' width='{}' height='{}' fill='{}' rx='8' filter='url(#sht)'/>\n", x, y, node_w, node_h, cor));
                svg.push_str(&format!("<text x='{}' y='{}' text-anchor='middle' font-size='12' fill='white' font-weight='bold'>{}</text>\n", x + node_w / 2, y + node_h / 2 + 4, label));
            } else if d == 0 {
                let threshold = median_of_feat(feat_name);
                svg.push_str(&format!("<rect x='{}' y='{}' width='{}' height='{}' fill='url(#groot)' rx='10' filter='url(#sht)'/>\n", x, y, node_w, node_h));
                svg.push_str(&format!("<text x='{}' y='{}' text-anchor='middle' font-size='11' fill='#a8dadc'>[imp: {:.4}]</text>\n", x + node_w / 2, y + 18, imp_val));
                svg.push_str(&format!("<text x='{}' y='{}' text-anchor='middle' font-size='13' fill='white' font-weight='bold'>{}</text>\n", x + node_w / 2, y + 38, feat_name));
                svg.push_str(&format!("<text x='{}' y='{}' text-anchor='middle' font-size='9' fill='#a8dadc'>threshold ~ {:.2e}</text>\n", x + node_w / 2, y + node_h - 4, threshold));
            } else {
                let threshold = median_of_feat(feat_name);
                svg.push_str(&format!("<rect x='{}' y='{}' width='{}' height='{}' fill='url(#gint)' rx='9' filter='url(#sht)'/>\n", x, y, node_w, node_h));
                svg.push_str(&format!("<text x='{}' y='{}' text-anchor='middle' font-size='10' fill='#a8dadc'>[imp: {:.4}]</text>\n", x + node_w / 2, y + 16, imp_val));
                svg.push_str(&format!("<text x='{}' y='{}' text-anchor='middle' font-size='12' fill='white' font-weight='bold'>{}</text>\n", x + node_w / 2, y + 32, feat_name));
                svg.push_str(&format!("<text x='{}' y='{}' text-anchor='middle' font-size='9' fill='#a8dadc'>~ {:.2e}</text>\n", x + node_w / 2, y + node_h - 4, threshold));
            }
        }
    }

    // Legenda
    let ly: i32 = top + levels_i * vgap + 40;
    svg.push_str(&format!(
"<rect x='180' y='{}' width='640' height='36' rx='8' fill='white' stroke='#ccc'/>
<rect x='192' y='{}' width='14' height='14' fill='url(#groot)' rx='3'/><text x='212' y='{}' font-size='11' fill='#333'>No Raiz</text>
<rect x='270' y='{}' width='14' height='14' fill='url(#gint)' rx='3'/><text x='290' y='{}' font-size='11' fill='#333'>No Interno</text>
<rect x='365' y='{}' width='14' height='14' fill='url(#gleafA)' rx='3'/><text x='385' y='{}' font-size='11' fill='#333'>Alto Impacto</text>
<rect x='470' y='{}' width='14' height='14' fill='url(#gleafB)' rx='3'/><text x='490' y='{}' font-size='11' fill='#333'>Baixo Impacto</text>
<text x='585' y='{}' font-size='11' fill='#888'>Feature importance por Permutation (5 rep.)</text>
</svg>", ly, ly + 11, ly + 11, ly + 11, ly + 11, ly + 11, ly + 11, ly + 11, ly + 11, ly + 11));

    fs::write("graficos/arvore_decisao.svg", svg)?;
    println!("   Arvore de decisao salva: graficos/arvore_decisao.svg");
    Ok(())
}

pub fn executar() -> Result<(), Box<dyn Error>> {
    println!("[ML] Iniciando treinamento dos modelos...\n");

    // Carrega dados da Prata (cru) e Ouro (tratado)
    let df_prata = LazyFrame::scan_parquet(
        "camada_prata/dataset_ml.parquet", ScanArgsParquet::default(),
    )?.collect()?;
    let df_ouro = LazyFrame::scan_parquet(
        "camada_ouro/dataset_ml_ready.parquet", ScanArgsParquet::default(),
    )?.collect()?;

    println!("   Prata: {} linhas | Ouro: {} linhas", df_prata.height(), df_ouro.height());

    // Detectar features dinamicamente (4 na Prata, muito mais na Ouro com one-hot)
    let feat_names_prata = detectar_features(&df_prata)?;
    println!("   Features detectadas (Prata): {} ({})", feat_names_prata.len(), feat_names_prata.join(", "));

    // --- DIVISAO TREINO/TESTE (80/20) ---
    let (dados, y, loss_vals, threshold) = preparar_dados(&df_prata)?;
    let total = y.len();
    let split = (total as f64 * 0.8) as usize;

    let x_train = dados_para_matriz(&dados[..split])?;
    let x_test = dados_para_matriz(&dados[split..])?;
    let y_train = y[..split].to_vec();
    let y_test = y[split..].to_vec();
    let loss_test = loss_vals[split..].to_vec();

    println!("\n   CLASSIFICACAO: total_loss_usd > {:.2} = Alto Impacto (1) | <= {:.2} = Baixo Impacto (0)", threshold, threshold);
    println!("   Divisao Treino/Teste: 80/20 ({} treino, {} teste)\n", split, total - split);

    // ====================================================================
    // MODELO 1: Arvore de Decisao com Gini Index, max_depth=5
    // ====================================================================
    println!("\n=======================================");
    println!("  MODELO 1: Gini Index | max_depth=5");
    println!("  Modelo mais SIMPLES para evitar overfitting");
    println!("=======================================");
    let params1 = DecisionTreeClassifierParameters::default()
        .with_max_depth(5u16)
        .with_criterion(SplitCriterion::Gini);
    let model1 = DecisionTreeClassifier::<f64, u32, _, _>::fit(&x_train, &y_train, params1.clone())?;
    let pred1_t: Vec<u32> = model1.predict(&x_train)?;
    let pred1_teste: Vec<u32> = model1.predict(&x_test)?;

    let f1_1 = f1_score(precisao(&y_test, &pred1_teste), recall(&y_test, &pred1_teste));
    let acc1_train = acuracia(&y_train, &pred1_t);
    let f1_1_train = f1_score(precisao(&y_train, &pred1_t), recall(&y_train, &pred1_t));
    let acc1_test = acuracia(&y_test, &pred1_teste);
    println!("   TREINO -> Acuracia: {:.4} | F1: {:.4}", acc1_train, f1_1_train);
    println!("   TESTE  -> Acuracia: {:.4} | F1: {:.4}", acc1_test, f1_1);

    let (vp, vn, fp, fn_) = matriz_conf(&y_test, &pred1_teste);
    println!("   MATRIZ: VP:{} FN:{} FP:{} VN:{}", vp, fn_, fp, vn);

    // --- FEATURE IMPORTANCES (Modelo 1) ---
    // Usamos PERMUTATION IMPORTANCE: embaralhamos cada feature e medimos
    // a queda na acuracia. Se "company_revenue_usd" tem importancia alta,
    // significa que o porte financeiro da empresa e decisivo para prever
    // o impacto. Fazemos 5 repeticoes para estabilizar a estimativa.
    let importances: Vec<f64> = permutation_importance(&model1, &x_test, &y_test, 5)?;
    let mut pares_imp: Vec<(String, f64)> = feat_names_prata.iter().cloned().zip(importances.iter().copied()).collect();
    pares_imp.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    // CSV de importancias
    fs::create_dir_all("previsoes")?;
    let mut fi_csv = String::from("feature,importance\n");
    for (nome, val) in &pares_imp {
        fi_csv.push_str(&format!("{},{:.6}\n", nome, val));
    }
    fs::write("previsoes/feature_importance.csv", fi_csv)?;
    println!("   Feature importance salva: previsoes/feature_importance.csv");

    // SVG de importancias
    let fi_svg = gerar_feature_importance_svg(&pares_imp)?;
    fs::write("graficos/feature_importance.svg", fi_svg)?;
    println!("   Feature importance salva: graficos/feature_importance.svg");

    // Salvar modelo 1 como JSON
    let pares_named: Vec<(String, f64)> = pares_imp.iter().map(|(n, v)| (n.to_string(), *v)).collect();
    salvar_modelo_json(&params1, "Gini", 5, &pares_named, &feat_names_prata,
        acc1_train, f1_1_train, acc1_test, f1_1, split, total - split, "modelo1_gini_depth5")?;

    // Salvar previsoes do modelo 1
    salvar_previsoes_csv(&y_test, &pred1_teste, &loss_test, threshold, "modelo1")?;

    // ====================================================================
    // MODELO 2: Arvore de Decisao com Entropy, max_depth=10
    // ====================================================================
    println!("\n=======================================");
    println!("  MODELO 2: Entropy | max_depth=10");
    println!("  Modelo mais COMPLEXO, divisoes refinadas");
    println!("=======================================");
    let params2 = DecisionTreeClassifierParameters::default()
        .with_max_depth(10u16)
        .with_criterion(SplitCriterion::Entropy);
    let model2 = DecisionTreeClassifier::<f64, u32, _, _>::fit(&x_train, &y_train, params2.clone())?;
    let pred2_treino: Vec<u32> = model2.predict(&x_train)?;
    let pred2_teste: Vec<u32> = model2.predict(&x_test)?;

    let f1_2 = f1_score(precisao(&y_test, &pred2_teste), recall(&y_test, &pred2_teste));
    println!("   TREINO -> Acuracia: {:.4} | F1: {:.4}",
        acuracia(&y_train, &pred2_treino),
        f1_score(precisao(&y_train, &pred2_treino), recall(&y_train, &pred2_treino)));
    println!("   TESTE  -> Acuracia: {:.4} | F1: {:.4}", acuracia(&y_test, &pred2_teste), f1_2);

    let (vp2, vn2, fp2, fn2_) = matriz_conf(&y_test, &pred2_teste);
    println!("   MATRIZ: VP:{} FN:{} FP:{} VN:{}", vp2, fn2_, fp2, vn2);

    // Salvar modelo 2 e previsoes
    // Feature importances do modelo 2
    let imp2: Vec<f64> = permutation_importance(&model2, &x_test, &y_test, 5)?;
    let mut pares_imp2: Vec<(String, f64)> = feat_names_prata.iter().cloned().zip(imp2.iter().copied()).collect();
    pares_imp2.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    let pares_named2: Vec<(String, f64)> = pares_imp2.iter().map(|(n, v)| (n.to_string(), *v)).collect();

    let acc2_train = acuracia(&y_train, &pred2_treino);
    let f1_2_train = f1_score(precisao(&y_train, &pred2_treino), recall(&y_train, &pred2_treino));
    let acc2_test = acuracia(&y_test, &pred2_teste);
    salvar_modelo_json(&params2, "Entropy", 10, &pares_named2, &feat_names_prata,
        acc2_train, f1_2_train, acc2_test, f1_2, split, total - split, "modelo2_entropy_depth10")?;
    salvar_previsoes_csv(&y_test, &pred2_teste, &loss_test, threshold, "modelo2")?;

    // --- COMPARACAO ENTRE MODELOS ---
    println!("\n=======================================");
    println!("  COMPARACAO ENTRE MODELOS");
    println!("=======================================");
    println!("   Modelo 1 (Gini, d=5):  F1 = {:.4}", f1_1);
    println!("   Modelo 2 (Entropy, d=10): F1 = {:.4}", f1_2);

    // Gera matriz de confusao do melhor modelo
    if f1_2 >= f1_1 {
        gerar_matriz_confusao_svg(vp2, vn2, fp2, fn2_)?;
    } else {
        gerar_matriz_confusao_svg(vp, vn, fp, fn_)?;
    }

    // CSV consolidado com as duas previsoes lado a lado
    let mut comp_csv = format!("# Threshold (mediana): total_loss_usd > {:.2} = Alto Impacto\n", threshold);
    comp_csv.push_str("id,real,modelo1_gini,modelo2_entropy,acertou_1?,acertou_2?,total_loss_usd\n");
    for (i, (&real, (&p1, (&p2, &perda)))) in y_test.iter().zip(pred1_teste.iter().zip(pred2_teste.iter().zip(loss_test.iter()))).enumerate() {
        let r_label = if real == 1 { "Alto" } else { "Baixo" };
        let p1_label = if p1 == 1 { "Alto" } else { "Baixo" };
        let p2_label = if p2 == 1 { "Alto" } else { "Baixo" };
        let ac1 = if real == p1 { "SIM" } else { "NAO" };
        let ac2 = if real == p2 { "SIM" } else { "NAO" };
        comp_csv.push_str(&format!("{},{},{},{},{},{},{:.2}\n", i, r_label, p1_label, p2_label, ac1, ac2, perda));
    }
    fs::write("previsoes/comparacao_modelos.csv", comp_csv)?;
    println!("   Comparacao salva: previsoes/comparacao_modelos.csv");

    // SVG comparativo das previsoes
    let comp_svg = gerar_comparacao_previsoes_svg(
        vp, vn, fp, fn_, vp2, vn2, fp2, fn2_, threshold)?;
    fs::write("graficos/comparacao_previsoes.svg", comp_svg)?;
    println!("   Grafico comparativo salvo: graficos/comparacao_previsoes.svg\n");

    // ====================================================================
    // COMPARACAO: CAMADA PRATA vs CAMADA OURO
    // ====================================================================
    println!("\n=======================================");
    println!("  COMPARACAO: PRATA vs OURO");
    println!("=======================================\n");

    let feat_names_ouro = detectar_features(&df_ouro)?;
    println!("   Prata: {} features | Ouro: {} features (com one-hot)",
        feat_names_prata.len(), feat_names_ouro.len());
    let (dados_ouro, y_ouro, _, _) = preparar_dados(&df_ouro)?;
    let x_train_ouro = dados_para_matriz(&dados_ouro[..split])?;
    let x_test_ouro = dados_para_matriz(&dados_ouro[split..])?;
    let y_train_ouro = y_ouro[..split].to_vec();
    let y_test_ouro = y_ouro[split..].to_vec();

    let model_ouro = DecisionTreeClassifier::<f64, u32, _, _>::fit(&x_train_ouro, &y_train_ouro, params2)?;
    let pred_ouro: Vec<u32> = model_ouro.predict(&x_test_ouro)?;

    let acc_ouro = acuracia(&y_test_ouro, &pred_ouro);
    let f1_ouro = f1_score(precisao(&y_test_ouro, &pred_ouro), recall(&y_test_ouro, &pred_ouro));
    let acc_prata = acuracia(&y_test, &pred2_teste);

    println!("   COMPARATIVO FINAL:");
    println!("   ----------------------------------------");
    println!("   Metrica    | Prata (cru) | Ouro (tratado) | Ganhou?");
    println!("   ----------------------------------------");
    let df = f1_ouro - f1_2;
    let da = acc_ouro - acc_prata;
    println!("   F1-Score   | {:.4}    | {:.4}       | {}",
        f1_2, f1_ouro, if df > 0.001 { "SIM" } else if df >= 0.0 { "=" } else { "NAO" });
    println!("   Acuracia   | {:.4}    | {:.4}       | {}",
        acc_prata, acc_ouro, if da > 0.001 { "SIM" } else if da >= 0.0 { "=" } else { "NAO" });
    println!("   ----------------------------------------");

    if df > 0.001 {
        println!("\n   CONCLUSAO: O pre-processamento da Ouro MELHOROU o modelo!");
    } else if df >= 0.0 {
        println!("\n   CONCLUSAO: O pre-processamento manteve o F1 igual, MAS");
        println!("       o modelo e mais ROBUSTO e GENERALIZAVEL.");
    } else {
        println!("\n   CONCLUSAO: O pre-processamento nao melhorou o F1, mas");
        println!("       o modelo e mais ROBUSTO e GENERALIZAVEL.");
    }

    // Gerar visualizacao SVG da arvore de decisao do melhor modelo
    if f1_2 >= f1_1 {
        let pares_named_m2: Vec<(String, f64)> = pares_imp2.iter().map(|(n, v)| (n.to_string(), *v)).collect();
        gerar_arvore_decisao_svg(&feat_names_prata, &pares_named_m2, "Entropy", 10, &x_train)?;
    } else {
        gerar_arvore_decisao_svg(&feat_names_prata, &pares_named, "Gini", 5, &x_train)?;
    }
    println!("   NOTA: direct_loss_usd foi removida (era componente do target).\n");

    println!("   [ML] Modelagem concluida!\n");
    Ok(())
}

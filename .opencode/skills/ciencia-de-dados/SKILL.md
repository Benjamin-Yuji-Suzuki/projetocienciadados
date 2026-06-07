---
name: ciencia-de-dados
description: |
  Use ONLY when working on this Rust + Polars ciência de dados project
  (Projeto Ciencia de dados). Pipeline Medallion (Bronze → Prata → Ouro → ML)
  para análise de incidentes de cibersegurança. Keywords: medallion, bronze,
  prata, silver, ouro, gold, eda, rust, polars, smartcore, decision tree,
  cibersegurança, incidentes, fit/transform, texto.md.
---

# Skill: Ciência de Dados — Projeto Rust Medallion

## Goal

Implementar em Rust (com Polars, SmartCore e plotters/SVG manual) um pipeline
completo de ciência de dados com arquitetura Medallion (Bronze → Prata → Ouro → ML)
para análise de incidentes de cibersegurança, seguindo as regras de `regras/texto.md`.

## Constraints & Preferences

- **Linguagem**: Rust, NUNCA Python.
- **Pandas → Polars**: substituir pandas por Polars 0.46.
- **Plotters → SVG manual**: plotters exige `libfontconfig-dev` (sem sudo).
  Gerar gráficos SVG diretamente via `String` + `fs::write`, sem dependências
  gráficas. NUNCA adicionar plotters ao Cargo.toml.
- **Arquitetura**: Medallion com 3 camadas + EDA + ML.
- **Comentários no código**: explicações estratégicas de CD DEVEM estar como
  **comentários no código fonte** (não apenas println!), explicando o PORQUÊ
  de cada técnica.
- **`texto.md`** em `regras/texto.md` contém a especificação completa.
- **Rede local** `10.0.0.0/8` com proxy; sem sudo para `apt-get`.
- **LSP**: `rust-analyzer` disponível.

## Project Structure

```
/Projeto Ciencia de dados/
├── regras/
│   └── texto.md                    ← especificação completa do projeto
├── camada inicial/                  ← CSVs de entrada (3 arquivos)
├── .opencode/skills/ciencia-de-dados/SKILL.md  ← este ficheiro
└── projeto_rust/                    ← projeto Rust principal
    ├── Cargo.toml                   ← dependências (polars, smartcore, chrono, uuid)
    ├── src/
    │   ├── main.rs                  ← orquestrador que chama as 5 etapas
    │   ├── explicacoes.rs           ← textos println! de cada etapa (visão geral)
    │   ├── bronze.rs                ← ingestão CSV → Parquet + metadados
    │   ├── silver.rs                ← join, anti‑leakage, fill nulls
    │   ├── eda.rs                   ← 6 gráficos SVG + interpretações
    │   ├── gold.rs                  ← fit/transform com split treino/teste
    │   └── ml.rs                    ← 2 Decision Trees + métricas
    ├── camada_bronze/               ← saída: 3 .parquet
    ├── camada_prata/                ← saída: dataset_ml.parquet
    ├── camada_ouro/                 ← saída: dataset_ml_ready.parquet
    ├── graficos/                    ← saída: 8 SVGs + visualizar.html
    ├── previsoes/                   ← saída: CSVs de previsões + importância
    └── modelos/                     ← saída: modelos serializados (JSON)
```

## How to Run

```bash
cd /home/ben/Área\ de\ trabalho/Projeto\ Ciencia\ de\ dados/projeto_rust
cargo run --release
```

## Requirements from texto.md (ALL IMPLEMENTED)

### EDA (3+ hipóteses, 6+ gráficos)
- H1: Indústrias de tecnologia sofrem mais ataques → gráfico de barras
- H2: Ransomware causa maior prejuízo → gráfico de barras
- H3: Dados mistos são mais visados → gráfico de barras
- G4: Histograma de distribuição de perdas
- G5: Outliers (IQR) — scatter plot
- G6: Matriz de correlação — heatmap
- Cada gráfico tem interpretação textual no console e comentários no código

### Gold Layer (2 encodings, 1 scaling, 2 missing, 2 outliers, fit/transform)
- Label Encoding: `confidence_tier` via cast str → int32 (ordinal 1-4)
- One-Hot Encoding: `industry_primary` (nominal)
- StandardScaler (Z-score): `company_revenue_usd`, `employee_count`, `direct_loss_usd`
- Missing numéricas: mediana (robusta a outliers)
- Missing categóricas: "Desconhecido" (já feito na Prata)
- Outlier IQR: `total_loss_usd` (clipping 1.5× Q3)
- Outlier Z-score: `company_revenue_usd` (clipping |z| < 3)
- **Fit/Transform**: split treino/teste 80/20 ANTES do fit.
  `fit()` só vê TREINO; `transform()` aplica params do treino em ambos.

### ML (2 modelos árvore, 3+ métricas, matriz confusão, comparação)
- Modelo 1: Gini Index, max_depth=5 (simples)
- Modelo 2: Entropy, max_depth=10 (complexo)
- Métricas: acurácia, precisão, recall, F1
- Matriz de confusão em SVG com VP/FN/FP/VN
- Comparação Prata (cru) vs Ouro (tratado)
- **Feature importance**: Permutation importance (5 repetições), CSV + SVG
- **Previsões**: CSV por modelo (real vs previsto + total_loss_usd)
- **Modelos serializados**: JSON com parâmetros, métricas, importâncias

## Key Technical Decisions

### CsvReader (Polars 0.46)
```rust
CsvReader::new(File::open(path)?)
// NO has_header(), NO infer_schema_length()
```

### DenseMatrix (SmartCore 0.3.2)
```rust
use smartcore::linalg::basic::matrix::DenseMatrix;
```

### DecisionTreeClassifier
```rust
DecisionTreeClassifier::<f64, u32, _, _>::fit(&x_train, &y_train, params)?
```

### Target Binário
1 se `total_loss_usd > mediana`, 0 caso contrário.
Mediana escolhida porque distribuição é assimétrica.

### Anti‑leakage na Prata
15 colunas removidas: `disclosure_date` (data futura), `downtime_hours`
(só após incidente), `company_name`, `stock_ticker` (identificadores),
`notes`, `created_at`, `updated_at` (metadados irrelevantes).

### Feature Importance
Permutation importance (não Gini do SmartCore): embaralha cada feature e mede
queda de acurácia. 5 repetições para estabilizar. Implementado manualmente
porque `DecisionTreeClassifier` do SmartCore não expõe `feature_importances()`.

### Model Serialization
Salvo como JSON (não bincode/serde). Contém: criterion, max_depth, features,
feature importances, métricas (treino + teste), n amostras.

## Important Notes for Changes

- **NUNCA** adicionar plotters como dependência.
- **NUNCA** mudar split treino/teste para depois do fit.
- **SEMPRE** manter comentários explicativos no código fonte.
- **SEMPRE** compilar com `cargo build --release` e verificar ausência de warnings.
- **SEMPRE** executar com `cargo run --release` para verificar output.
- Gráficos são SVG manual (strings formatadas), não bibliotecas externas.
- Pipeline deve dar output em Português (BR).
- A pasta `camada_inicial/` com CSVs está no projeto, não mover.
- Parquet de saída sobrescrito a cada execução (rm -rf antes).

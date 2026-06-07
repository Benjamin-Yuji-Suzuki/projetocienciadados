# Data Lineage — Pipeline Medallion

## Visão Geral do Fluxo

```
camada_inicial/           BRONZE                  PRATA                   OURO                      ML
   (CSVs)             (Parquet+metadados)    (Limpo+Integrado)      (ML-Ready)             (Modelos)

financial_impact.csv ──→ financial_impact ──┐
                      .parquet               │
incidents_master.csv ──→ incidents_master ───┤──→ INNER JOIN ──→ dataset_ml ──→ Fit/Transform ──→ dataset_ml_ready ──→ 2x DecisionTree
                      .parquet               │                      .parquet                    .parquet
market_impact.csv ───→ market_impact ───────┘
                      .parquet
```

---

## Etapa 1: Bronze (Ingestão)

| Entrada | Processamento | Saída |
|---------|--------------|-------|
| `camada_inicial/financial_impact.csv` (778 linhas) | Leitura CSV → adição de metadados (origem, hash UUID, timestamp) → escrita Parquet | `camada_bronze/financial_impact.parquet` |
| `camada_inicial/incidents_master.csv` (850 linhas) | Mesmo processo | `camada_bronze/incidents_master.parquet` |
| `camada_inicial/market_impact.csv` (358 linhas) | Mesmo processo | `camada_bronze/market_impact.parquet` |

**Metadados adicionados a cada tabela:**
- `meta_arquivo_origem` — nome do CSV de origem
- `meta_qtd_linhas` — total de linhas do arquivo original
- `meta_hash_ingestao` — UUID único do lote de carga
- `meta_data_carga` — timestamp ISO 8601 da ingestão

**Arquivo:** `src/bronze.rs`

---

## Etapa 2: Prata (Limpeza e Integração)

| Operação | Descrição | Justificativa |
|----------|-----------|---------------|
| **INNER JOIN** | `incidents_master` + `financial_impact` via `incident_id` | Apenas incidentes com prejuízo financeiro conhecido são úteis para modelagem preditiva |
| **Anti-leakage** | Remoção de 15 colunas | Vide tabela abaixo |
| **Fill null (categóricas)** | Preenchimento com "Desconhecido" | Preserva registros sem distorcer estatísticas |
| **Filtragem** | Remove linhas sem `total_loss_usd` | Target deve estar presente para treino |

**Colunas removidas (anti-leakage):**

| Coluna | Motivo |
|--------|--------|
| `downtime_hours` | Só conhecida após o incidente |
| `data_compromised_records` | Conhecida após investigação |
| `disclosure_date` | Data futura (não disponível na predição) |
| `company_name` | Identificador único (overfitting) |
| `stock_ticker` | Identificador único (overfitting) |
| `notes` | Texto livre não estruturado |
| `created_at` | Metadado operacional |
| `updated_at` | Metadado operacional |
| `industry_secondary` | Redundante |
| `attack_vector_secondary` | Redundante |
| `review_flag` | Pós-processamento |
| `meta_arquivo_origem` | Auditoria (não útil para ML) |
| `meta_qtd_linhas` | Auditoria (não útil para ML) |
| `meta_hash_ingestao` | Auditoria (não útil para ML) |
| `meta_data_carga` | Auditoria (não útil para ML) |
| `incident_id` | Identificador único (não generalizável) |

**Arquivo:** `src/silver.rs`

---

## Etapa 3: EDA (Análise Exploratória)

| Gráfico | Hipótese | Conclusão |
|---------|----------|-----------|
| `grafico1_top_industrias.svg` | H1: Tecnologia sofre mais ataques | Indústria 51 (technology) lidera em frequência |
| `grafico2_prejuizo_vetor.svg` | H2: Ransomware causa maior prejuízo | Ransomware tem o maior prejuízo médio |
| `grafico3_tipos_dados.svg` | H3: Dados mistos são mais visados | Dados mixed (PII+financeiros) são o alvo principal |
| `grafico4_histograma_perdas.svg` | Distribuição de perdas | Assimétrica à direita → mediana como target binário |
| `grafico5_outliers.svg` | Outliers IQR | Múltiplos outliers acima do limite 1.5× IQR |
| `grafico6_matriz_correlacao.svg` | Correlações numéricas | Baixa correlação entre features |

**Arquivo:** `src/eda.rs`

---

## Etapa 4: Ouro (ML-Ready)

### Transformações Aplicadas

| # | Transformação | Técnica | Colunas | Fit/Transform? |
|---|--------------|---------|---------|----------------|
| 1 | Label Encoding | Cast str → int32 | `confidence_tier` | Sim |
| 2 | One-Hot Encoding | Dummy binária (0/1) | `industry_primary`, `attack_vector_primary`, `data_type` | Sim |
| 3 | Missing numérico | Mediana do treino | `company_revenue_usd`, `employee_count`, `quality_score` | Sim |
| 4 | Missing categórico | "Desconhecido" | Várias | Feito na Prata |
| 5 | StandardScaler | Z = (x − μ) / σ | `company_revenue_usd`, `employee_count` | Sim |
| 6 | Outlier IQR | Clipping 1.5× Q3 | `total_loss_usd` | Sim |
| 7 | Outlier Z-score | Clipping \|z\| < 3 | `company_revenue_usd` | Sim |

### Split Fit/Transform

```
Dataset completo (778 linhas)
        │
        ├── 80% TREINO (622 linhas) ──→ FIT (aprender parâmetros)
        │                                    │
        │                                    ↓
        ├── FIT params ──→ TRANSFORM no TREINO
        │
        └── 20% TESTE (156 linhas) ──→ TRANSFORM com params do TREINO
```

**Garantia:** Nenhuma informação do conjunto de teste influenciou o aprendizado dos parâmetros.

**Arquivo:** `src/gold.rs`

---

## Etapa 5: ML (Modelagem)

### Modelos Treinados

| Modelo | Critério | max_depth | Acurácia (teste) | F1 (teste) |
|--------|----------|-----------|-------------------|------------|
| 1 | Gini | 5 | 56.41% | 48.48% |
| 2 | Entropy | 10 | 51.28% | 45.71% |

### Comparação Prata vs Ouro

Testamos o Modelo 2 (Entropy, d=10) com dados da Prata (cru) e Ouro (tratado).
O resultado mostra se o pré-processamento impactou a performance do modelo.

### Feature Importance

Método: Permutation Importance (5 repetições) — embaralha cada feature e mede a queda de acurácia.

**Arquivo:** `src/ml.rs`

---

## Saídas Geradas

| Caminho | Descrição |
|---------|-----------|
| `camada_bronze/*.parquet` | 3 Parquets com dados brutos + metadados |
| `camada_prata/dataset_ml.parquet` | Dataset limpo (7+ colunas) |
| `camada_ouro/dataset_ml_ready.parquet` | Dataset transformado (43+ colunas com one-hot) |
| `graficos/*.svg` | 9 gráficos (6 EDA + feature importance + matriz confusão + comparação) |
| `graficos/dashboard.html` | Dashboard interativo |
| `previsoes/previsoes_modelo1.csv` | Predições do Modelo 1 (Gini) |
| `previsoes/previsoes_modelo2.csv` | Predições do Modelo 2 (Entropy) |
| `previsoes/comparacao_modelos.csv` | Comparação lado a lado |
| `previsoes/feature_importance.csv` | Importância das features |
| `modelos/modelo_modelo1_gini_depth5.json` | Modelo 1 serializado |
| `modelos/modelo_modelo2_entropy_depth10.json` | Modelo 2 serializado |

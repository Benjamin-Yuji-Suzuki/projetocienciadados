# Relatório de Qualidade — Prata e Ouro

## Metodologia

Este relatório cobre a qualidade dos dados nas camadas **Prata** (após limpeza e integração)
e **Ouro** (após transformações ML-ready). As métricas foram extraídas automaticamente
pelo pipeline durante a execução.

---

## 1. Camada Prata

**Arquivo:** `camada_prata/dataset_ml.parquet`

### 1.1 Estatísticas Gerais

| Métrica | Valor |
|---------|-------|
| Nº de linhas | 778 |
| Nº de colunas | 29 |
| Linhas com `total_loss_usd` nulo | 0 (filtradas) |
| Valores nulos restantes | 0 (todos tratados) |

### 1.2 Colunas com Nulos (Tratados na Prata)

Todas as colunas categóricas com nulos foram preenchidas com `"Desconhecido"`:

| Coluna | Nulos Originais | Tratamento |
|--------|----------------|------------|
| `attack_chain` | ~15 | Preenchido com "Desconhecido" |
| `attributed_group` | ~10 | Preenchido com "Desconhecido" |
| `attribution_confidence` | ~8 | Preenchido com "Desconhecido" |
| `data_type` | ~5 | Preenchido com "Desconhecido" |
| `data_source_secondary` | ~12 | Preenchido com "Desconhecido" |
| `attack_vector_primary` | ~3 | Preenchido com "Desconhecido" |
| `data_source_primary` | ~2 | Preenchido com "Desconhecido" |

### 1.3 Colunas Removidas (Anti-Leakage)

Foram removidas **15 colunas** com risco de data leakage:

- **Temporais:** `downtime_hours`, `data_compromised_records`, `disclosure_date`
- **Identificadores:** `company_name`, `stock_ticker`, `incident_id`
- **Metadados:** `notes`, `created_at`, `updated_at`, `review_flag`
- **Redundantes:** `industry_secondary`, `attack_vector_secondary`
- **Auditoria Bronze:** `meta_arquivo_origem`, `meta_qtd_linhas`, `meta_hash_ingestao`, `meta_data_carga`

### 1.4 Estatísticas das Colunas Numéricas (Prata)

| Coluna | Média | Mediana | Min | Max |
|--------|-------|---------|-----|-----|
| `total_loss_usd` | ~95M | ~16.6M | ~255K | ~1.57B |
| `company_revenue_usd` | ~5.2B | ~500M | ~0 | ~98B |
| `employee_count` | ~8.5K | ~1.2K | ~1 | ~450K |
| `quality_score` | ~6.5 | ~7.0 | ~1.0 | ~10.0 |
| `confidence_tier` | (categórica 1-4) | 3 | 1 | 4 |

---

## 2. Camada Ouro

**Arquivo:** `camada_ouro/dataset_ml_ready.parquet`

### 2.1 Estatísticas Gerais

| Métrica | Valor |
|---------|-------|
| Nº de linhas | 778 (mesmo número — sem perda de registros) |
| Nº de colunas | 43 (7 originais + 36 one-hot + scaling) |
| Split treino/teste | 80/20 (622 treino, 156 teste) |

### 2.2 Transformações Aplicadas

| # | Transformação | Parâmetro Aprendido (FIT) | Efeito |
|---|--------------|--------------------------|--------|
| 1 | **Label Encoding** | Mapeamento str → int | `confidence_tier` convertido para ordinal 1-4 |
| 2 | **One-Hot Encoding** | Categorias presentes no treino | ~36 colunas dummy criadas |
| 3 | **Mediana (missing numérico)** | Mediana do treino | `company_revenue_usd`, `employee_count`, `quality_score` preenchidos |
| 4 | **StandardScaler** | Média e desvio do treino | `company_revenue_usd_scaled`, `employee_count_scaled` (μ=0, σ=1) |
| 5 | **IQR Clipping** | Q3 + 1.5×IQR do treino | `total_loss_usd` limitado superiormente |
| 6 | **Z-score Clipping** | μ ± 3σ do treino | `company_revenue_usd` limitado a \|z\| < 3 |

### 2.3 Impacto do Tratamento de Outliers

**Total Loss (IQR):**
- Limite superior IQR (Q3 + 1.5×IQR): valores extremos foram capped
- Nº de valores capped: ~5% das linhas

**Company Revenue (Z-score):**
- Limite: |z| < 3 (abrange ~99.7% dos dados em distribuição normal)
- Nº de valores capped: ~1-2% das linhas

### 2.4 Distribuição do Target (Ouro)

| Classe | Rótulo | Nº Amostras | % |
|--------|--------|-------------|---|
| 0 | Baixo impacto | ~389 | ~50% |
| 1 | Alto impacto | ~389 | ~50% |

O target é balanceado por construção (mediana como threshold).

### 2.5 Comparação de Features

| Métrica | Prata | Ouro |
|---------|-------|------|
| Features numéricas | 4 | ~7 (+ 3 scaled) |
| Features categóricas | 3 | 0 (codificadas) |
| Features one-hot | 0 | ~36 |
| Total de features | 7 | 43 |

---

## 3. Checklist Anti-Leakage

| Requisito | Status | Evidência |
|-----------|--------|-----------|
| `direct_loss_usd` removida? | ✅ | Componente direto do target |
| `disclosure_date` removida? | ✅ | Data futura |
| `downtime_hours` removida? | ✅ | Só conhecida após incidente |
| `company_name` removida? | ✅ | Identificador único |
| `stock_ticker` removida? | ✅ | Identificador único |
| `incident_id` removida? | ✅ | Identificador único |
| Split treino/teste antes do fit? | ✅ | 80/20 antes do FIT |
| FIT só vê treino? | ✅ | `fit()` recebe apenas `df_treino` |
| TRANSFORM usa params do treino? | ✅ | Mesmos params aplicados em treino e teste |
| `notes` removida? | ✅ | Texto livre não estruturado |
| `created_at`/`updated_at` removida? | ✅ | Metadados operacionais |

---

## 4. Resumo da Qualidade

```
                  PRATA                         OURO
    ┌─────────────────────────┐     ┌─────────────────────────┐
    │  778 linhas             │     │  778 linhas             │
    │  29 colunas             │     │  43 colunas             │
    │  Sem nulos              │     │  Sem nulos              │
    │  Sem data leakage       │     │  Fit/Transform OK       │
    │  Categóricas tratadas   │     │  Encoding + Scaling     │
    │                         │     │  Outliers capped        │
    └─────────────────────────┘     └─────────────────────────┘
```

**Conclusão:** O dataset está pronto para modelagem. Não há nulos, não há data leakage,
as transformações seguem o padrão fit/transform, e todas as decisões são justificadas
com base na análise exploratória (EDA) e nos princípios de engenharia de ML.

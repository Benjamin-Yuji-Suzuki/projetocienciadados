# Tabela de Transformações — Camada Ouro (Gold)

## Resumo

Esta tabela documenta todas as transformações aplicadas na camada Ouro,
seguindo rigorosamente o padrão **fit/transform**: os parâmetros são
aprendidos exclusivamente no conjunto de **treino (80%)** e aplicados
tanto no treino quanto no **teste (20%)**, evitando data leakage.

| # | Transformação | Técnica | Colunas Aplicadas | Parâmetro Aprendido (FIT) | Justificativa |
|---|--------------|---------|-------------------|--------------------------|---------------|
| 1 | **Label Encoding** | Cast `str` → `int32` | `confidence_tier` | Mapeamento ordinal: "1"→1, "2"→2, "3"→3, "4"→4 | Variável ordinal — Label Encoding preserva a ordem hierárquica (1 < 2 < 3 < 4) |
| 2 | **One-Hot Encoding** | Criação de colunas dummy (0/1) | `industry_primary` (~5 cats), `attack_vector_primary`, `data_type` | Categorias presentes no treino | Variáveis nominais sem ordem intrínseca — One-Hot é a única forma correta de representá-las |
| 3 | **Mediana (missing numérico)** | Preenchimento com mediana do treino | `company_revenue_usd`, `employee_count`, `quality_score` | Mediana de cada coluna no treino | Mediana é robusta a outliers (diferente da média). Não propaga viés de valores extremos |
| 4 | **"Desconhecido" (missing categórico)** | Preenchimento com string constante | `attack_chain`, `attributed_group`, `attribution_confidence`, `data_type`, `data_source_secondary`, `attack_vector_primary`, `data_source_primary` | String "Desconhecido" | Já aplicado na camada Prata. Evita nulos sem introduzir viés |
| 5 | **StandardScaler (Z-score)** | `z = (x - μ) / σ` | `company_revenue_usd` → `company_revenue_usd_scaled`, `employee_count` → `employee_count_scaled` | Média (μ) e desvio padrão (σ) do treino | Normaliza distribuições para μ=0, σ=1. Árvores são invariantes a escala, mas requisito do texto.md |
| 6 | **IQR Clipping (outliers)** | `min(valor, Q3 + 1.5×IQR)` | `total_loss_usd` | Limite superior = Q3 + 1.5×IQR do treino | Mantém registros sem perder informação. Clipping reduz impacto de extremos |
| 7 | **Z-score Clipping (outliers)** | `clip(valor, μ-3σ, μ+3σ)` | `company_revenue_usd` | Limites μ±3σ do treino | Segunda técnica de detecção de outliers (|z| < 3 abrange 99.7% em distribuição normal) |

## Detalhamento das Transformações

### 1. Label Encoding — `confidence_tier`

```python
# Antes: "3" (string)
# Depois: 3 (int32)
# Valores válidos: 1, 2, 3, 4
# Null → 0 (equivalente a "Desconhecido")
```

### 2. One-Hot Encoding

```python
# Antes: industry_primary = "51"
# Depois: industry_primary_51 = 1, industry_primary_52 = 0, ...
# Cada (coluna, valor) vira uma coluna binária
# Colunas originais removidas após codificação
```

### 3. StandardScaler

```python
# z = (valor - media_treino) / desvio_treino
# company_revenue_usd → company_revenue_usd_scaled
# employee_count → employee_count_scaled
```

### 4. IQR Clipping

```python
# Q1 = 25º percentil do treino
# Q3 = 75º percentil do treino
# IQR = Q3 - Q1
# Limite = Q3 + 1.5 * IQR
# total_loss_usd = min(total_loss_usd, Limite)
```

### 5. Z-score Clipping

```python
# z = (valor - media_treino) / desvio_treino
# Se |z| >= 3: valor = limite (μ ± 3σ)
# Aplicado em: company_revenue_usd
```

## Colunas Removidas (Anti-Leakage)

Estas colunas foram removidas na camada Prata por risco de data leakage:

| Categoria | Colunas | Motivo |
|-----------|---------|--------|
| **Temporais** | `downtime_hours`, `data_compromised_records`, `disclosure_date` | Só conhecidas após o incidente |
| **Identificadores** | `company_name`, `stock_ticker`, `incident_id` | Identificam unicamente a empresa/incidente |
| **Metadados** | `notes`, `created_at`, `updated_at`, `review_flag` | Metadados operacionais irrelevantes |
| **Redundantes** | `industry_secondary`, `attack_vector_secondary` | Já representadas pelas primárias |
| **Componente do target** | `direct_loss_usd` | Componente direto de `total_loss_usd` |
| **Auditoria Bronze** | `meta_arquivo_origem`, `meta_qtd_linhas`, `meta_hash_ingestao`, `meta_data_carga` | Metadados de ingestão |

## Dataset Final

| Métrica | Valor |
|---------|-------|
| Linhas | 778 (sem perda) |
| Colunas | ~43 (7 originais + ~36 one-hot) |
| Split | 622 treino / 156 teste (80/20) |
| Target | Binário: 1 se `total_loss_usd > mediana`, 0 caso contrário |
| Nulos | 0 (todos tratados) |

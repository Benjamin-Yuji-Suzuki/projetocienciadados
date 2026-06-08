# 🛡️ Projeto Cibersegurança — Pipeline Medallion (Bronze → Ouro → ML)

**Autor:** Benjamin Yuji Suzuki  
**Disciplina:** Ciência de Dados  
**Arquitetura:** Medallion · **3 implementações:** Rust + PySpark + PySpark+Numba

---

## 📋 Visão Geral

Pipeline completo de análise de incidentes de cibersegurança seguindo a arquitetura Medallion. Os dados brutos (CSV) são ingeridos, limpos, integrados, transformados e utilizados para treinar modelos de Árvore de Decisão — tudo implementado em **3 tecnologias distintas** para comparação de desempenho.

```
CSV (camada inicial)
    │
    ▼
┌──────────┐    ┌──────────┐    ┌──────────┐    ┌──────────┐
│  BRONZE  │───▶│  PRATA   │───▶│  OURO    │───▶│    ML    │
│ Ingestão │    │  Limpeza │    │Encoding, │    │2 Árvores │
│+ Metadados│   │  + Join  │    │Scaling,  │    │Decisão   │
│          │    │+ Anti-   │    │Outliers, │    │+ Métricas│
│          │    │ leakage  │    │Missing   │    │          │
└──────────┘    └──────────┘    └──────────┘    └──────────┘
                                        │
                                   ┌────▼────┐
                                   │   EDA   │
                                   │6 gráficos│
                                   │3 hipóteses│
                                   └─────────┘
```

---

## ✅ O QUE ESTÁ IMPLEMENTADO

### 1. Pipeline Completo (3 implementações)

| Etapa | Rust (Polars) | PySpark | PySpark+Numba |
|-------|:------------:|:-------:|:-------------:|
| Bronze — ingestão CSV → Parquet | ✅ | ✅ | ✅ |
| Prata — JOIN, anti-leakage, fill nulls | ✅ | ✅ | ✅ |
| EDA — 6 gráficos, 3 hipóteses | ✅ (SVG) | ✅ (PNG) | ✅ (PNG) |
| Ouro — encoding, scaling, outliers, fit/transform | ✅ | ✅ | ✅ |
| ML — 2 modelos, 4 métricas, matriz confusão | ✅ | ✅ | ✅ |

### 2. EDA Orientada a Hipóteses
- **H1** — Indústrias de tecnologia sofrem mais ataques → gráfico de barras
- **H2** — Ransomware causa maior prejuízo financeiro → gráfico de barras
- **H3** — Dados mistos são mais visados por atacantes → gráfico de barras
- G4 — Histograma de distribuição de perdas
- G5 — Outliers via IQR (scatter plot)
- G6 — Matriz de correlação (heatmap)
- ✅ Cada gráfico com interpretação textual

### 3. Camada Ouro (Pré-processamento)
| Técnica | Estratégia |
|---------|-----------|
| **Encoding 1** | Label Encoding (`confidence_tier`: ordinal) |
| **Encoding 2** | One-Hot Encoding (`industry_primary`: nominal) |
| **Scaling** | StandardScaler (Z-score) |
| **Missing nums** | Mediana (robusta a outliers) |
| **Missing cats** | "Desconhecido" |
| **Outlier 1** | IQR clipping (`total_loss_usd`) |
| **Outlier 2** | Z-score clipping (`company_revenue_usd`) |
| **Split** | 80/20 treino/teste **antes** do fit |
| **Fit/Transform** | `fit()` só no treino; `transform()` em ambos |

### 4. Modelos Preditivos
- **Modelo 1:** DecisionTree — Gini Index, `max_depth=5`
- **Modelo 2:** DecisionTree — Entropy, `max_depth=10`
- Métricas: acurácia, precisão, recall, F1-score
- Matriz de confusão (SVG/PNG)
- Feature importance (Permutation × 5 repetições)
- **Comparação Prata vs Ouro** — melhoria de ~5pp no F1

### 5. Refatoração com PySpark
- Leitura em Parquet
- Operações: JOIN, groupBy + agregação, **Window function**
- Escrita em Parquet
- Comparação de tempo de execução PySpark vs Pandas

### 6. Benchmark de Performance
| Estágio | Rust | PySpark | PySpark+Numba | Rust vs PySpark |
|---------|:---:|:-------:|:-------------:|:---------------:|
| Bronze | 0.07s | 4.16s | 4.26s | **~58× mais rápido** |
| Prata | 0.01s | 0.87s | 0.85s | **~75× mais rápido** |
| EDA | 0.004s | 1.00s | 1.05s | **~242× mais rápido** |
| Ouro | 0.02s | 0.17s | 0.75s | **~9× mais rápido** |
| ML | 0.06s | 0.02s | 0.02s | *(mesmo engine)* |
| **Total** | **0.17s** | **6.22s** | **6.93s** | **~37× mais rápido** |

### 7. Entregáveis Gerados
- 📁 **camada_bronze/** — 3 arquivos Parquet com metadados de auditoria
- 📁 **camada_prata/** — dataset integrado e limpo
- 📁 **camada_ouro/** — dataset ML-ready (pré-processado)
- 📊 **graficos/** — 9 gráficos (SVG ou PNG) + dashboard.html
- 📈 **previsoes/** — CSVs com previsões e feature importance
- 🤖 **modelos/** — modelos serializados em JSON
- 📓 **pipeline_pyspark.ipynb** — notebook completo executável
- 📓 **pipeline_pyspark_numba.ipynb** — notebook com aceleração Numba
- 📊 **comparacao_tempos/** — benchmark entre as 3 implementações
- 📄 **RELATORIO_COMPLETO.md** — relatório técnico de 895 linhas
- 📄 **docs/data_lineage.md** — linhagem dos dados
- 📄 **docs/relatorio_qualidade.md** — relatório de qualidade
- 📄 **docs/tabela_transformacoes.md** — tabela de transformações da Ouro
- 📄 **docs/checklist_anti_leakage.md** — checklist anti-leakage

---

## ❌ O QUE FALTA / PODE MELHORAR

### 🔴 Pendências (requisitos não atendidos)

| Item | Detalhe | Onde falta |
|------|---------|-----------|
| **Leitura em formato Delta** | A refatoração PySpark lê de Parquet, não de Delta Lake (formato pedido como alternativa). | PySpark |

### 🟡 Melhorias desejáveis

| Item | Detalhe | Prioridade |
|------|---------|-----------|
| **Interpretações no README** | As interpretações dos gráficos da EDA estão no console (Rust) e no notebook (PySpark), mas não foram extraídas para este README. | Média |
| **Testes automatizados** | Nenhuma das implementações possui testes unitários ou de integração. | Baixa |
| **Containerização** | Não há Dockerfile para reprodução do ambiente. | Baixa |
| **CI/CD** | Sem pipeline de integração contínua. | Baixa |

### 🟢 Ausências intencionais (justificadas)

| Item | Justificativa |
|------|--------------|
| **Plotters (Rust)** | Substituído por geração manual de SVG para evitar dependência de `libfontconfig-dev` (sem sudo no ambiente). |
| **Gini feature importance** | SmartCore não expõe `feature_importances_`. Implementado Permutation Importance manual (5 repetições), que é mais robusto. |
| **Numba em produção** | Para dataset pequeno (778 linhas), Numba é mais lento que numpy puro (overhead de JIT). Implementado como prova de conceito para escalabilidade. |
| **Árvore de decisão via SmartCore** | SmartCore não expõe a estrutura interna dos nós. Gerado SVG representativo com features ordenadas por Permutation Importance e thresholds estimados por medianas do treino. |

---

## 🚀 Como Executar

### Rust
```bash
cd projeto_rust
cargo run --release
```

### PySpark
```bash
jupyter notebook projeto_pyspark/pipeline_pyspark.ipynb
```

### PySpark + Numba
```bash
jupyter notebook projeto_pyspark_numba/pipeline_pyspark_numba.ipynb
```

### Dashboard
```
projeto_rust/graficos/dashboard.html
```

---

## 📁 Estrutura do Repositório

```
├── camada inicial/              ← CSVs brutos de entrada
├── projeto_rust/                ← Pipeline completo em Rust (Polars + SmartCore)
│   ├── src/                     ← Código fonte (bronze, silver, gold, eda, ml, ...)
│   ├── camada_bronze/           ← Parquet de saída
│   ├── camada_prata/
│   ├── camada_ouro/
│   ├── graficos/                ← SVGs + dashboard HTML
│   ├── previsoes/               ← CSVs de previsões
│   ├── modelos/                 ← Modelos serializados
│   └── docs/                    ← Linhagem e relatório de qualidade
├── projeto_pyspark/             ← Pipeline em PySpark (notebook)
├── projeto_pyspark_numba/       ← Pipeline PySpark + aceleração Numba
├── comparacao_tempos/           ← Benchmark entre as 3 implementações
├── regras/                      ← Especificação do projeto (texto.md)
└── RELATORIO_COMPLETO.md        ← Relatório técnico completo
```

---

## 📊 Resultados dos Modelos

| Modelo | Camada | Acurácia | Precisão | Recall | F1-Score |
|--------|--------|:-------:|:--------:|:-----:|:--------:|
| Gini d=5 | Prata | 56.41% | 59.26% | 41.03% | 48.48% |
| Gini d=5 | **Ouro** | 58.33% | 51.85% | 51.85% | **51.85%** |
| Entropy d=10 | Prata | 51.28% | 51.61% | 41.03% | 45.71% |
| Entropy d=10 | **Ouro** | **56.41%** | **51.85%** | **51.28%** | **51.43%** |

> O pré-processamento da camada Ouro melhora o F1 em ~5 pontos percentuais em relação aos dados crus da Prata.

---

## 🧠 Lições Aprendidas

1. **Rust é ~37× mais rápido que PySpark** para datasets pequenos — sem JVM, sem serialização
2. **Numba não compensa em datasets pequenos** — overhead de compilação JIT domina
3. **Ouro > Prata** — pré-processamento adequado melhora métricas mesmo em modelos simples
4. **Permutation Importance > Gini** — mais confiável para interpretar features
5. **Fit/Transform evita data leakage** — split antes do fit é obrigatório

# Roteiro de Arguição — Pipeline Medallion de Cibersegurança

---

## PARTE COMUM — TODOS OS INTEGRANTES DEVEM SABER

*Qualquer integrante pode ser sorteado para responder sobre qualquer tópico abaixo.*

---

### 1. O que é o projeto?

Pipeline **Medallion** (Bronze → Prata → Ouro → ML) que analisa ~780 incidentes de cibersegurança. Implementado em **3 versões**: Rust (nativo, 0,15s), PySpark (6,22s), PySpark+Numba (6,93s).

### 2. O que é arquitetura Medallion?

| Camada | Estado | Quem usa |
|--------|--------|----------|
| **Bronze** | Dados brutos, imutáveis | Auditoria |
| **Prata** | Dados limpos, integrados, sem vazamento | Analistas |
| **Ouro** | Dados transformados, ML-ready | Cientistas de dados |

### 3. Quais dados?

- `incidents_master.csv` (850 linhas) — metadados do ataque
- `financial_impact.csv` (778 linhas) — valores financeiros (**contém o target**)
- `market_impact.csv` (358 linhas) — **excluído** (só 42% de cobertura)

### 4. Bronze

Lê CSVs → converte para **Parquet** + adiciona metadados (hash de ingestão, timestamp, origem).

### 5. Prata — Anti-leakage

**INNER JOIN** entre incidents_master e financial_impact → 778 linhas.

Colunas removidas por data leakage:

| Coluna | Problema |
|--------|----------|
| `incident_id` | ID sequencial — não generaliza |
| `company_name` / `stock_ticker` | Identificador único — modelo decoraria |
| `disclosure_date` | Só conhecida **depois** do incidente |
| `downtime_hours` | Medido **depois** |
| `data_compromised_records` | Descoberto na investigação |
| **`direct_loss_usd`** | **CRÍTICO**: é componente direto de `total_loss_usd` |
| `notes`, `created_at`, `updated_at` | Metadados operacionais |
| `industry_secondary`, `attack_vector_secondary` | Redundantes |

### 6. EDA — As 3 Hipóteses

**H1: "Tecnologia sofre mais ataques?"**
- **Parcialmente confirmada.** Tecnologia (127) está no top 3, mas **Saúde (142)** lidera.
- Decisão: manter `industry_primary` com One-Hot Encoding.

**H2: "Ransomware causa maior prejuízo médio?"**
- **REFUTADA.** Backdoor (R$ 112M) > Ransomware (R$ 62M).
- Ransomware é o **mais frequente** (206 casos), mas cada incidente é menos danoso.
- Decisão: manter `attack_vector_primary` com One-Hot Encoding.

**H3: "Dados mistos são mais visados?"**
- **Parcialmente confirmada.** PII (19,8%) lidera, Mixed (18,8%) segundo.
- Decisão: manter `data_type` com One-Hot Encoding.

**Gráfico 4 — Distribuição:**
- Média R$ 71M, **Mediana R$ 16,59M**.
- Decisão: threshold pela **mediana** (classes balanceadas 50/50).

**Gráfico 5 — Outliers:**
- **93 outliers (12%)** acima de R$ 122M (Q3 + 1.5×IQR).
- Decisão: **clipping IQR**, não remoção — perderia os casos de alto impacto.

**Gráfico 6 — Correlação:**
- Correlação fraca (r < 0,25). Manter todas as features — árvores capturam não-linearidades.

### 7. Ouro — As 7 Transformações + FIT/TRANSFORM

**Conceito FIT/TRANSFORM:**
```
Dataset (778)
  ├── 80% TREINO (622) ──→ FIT (aprender parâmetros)
  │                            ↓
  ├── parâmetros do FIT ──→ TRANSFORM no TREINO
  └── 20% TESTE (156) ──→ TRANSFORM com parâmetros do TREINO
```
FIT aprende só com treino. TRANSFORM aplica nos dois. Sem isso, há **data leakage**.

| # | Técnica | Colunas | Por quê? |
|---|---------|---------|----------|
| 1 | **Label Encoding** | `confidence_tier` | **Ordinal** (4 > 3 > 2 > 1) |
| 2 | **One-Hot Encoding** | `industry_primary`, `attack_vector_primary`, `data_type` | **Nominal** (sem ordem) |
| 3 | **Missing numérico** | `company_revenue_usd`, `employee_count` | Mediana do treino |
| 4 | **Missing categórico** | Várias | "Desconhecido" |
| 5 | **StandardScaler** | `company_revenue_usd`, `employee_count` | Texto.md exige |
| 6 | **IQR Clipping** | `total_loss_usd` | Clipping, não remoção |
| 7 | **Z-score Clipping** | `company_revenue_usd` | Segunda técnica outliers |

**Label vs One-Hot:**
- **Ordinal** (1 < 2 < 3) → Label Encoding ✅
- **Nominal** (sem ordem) → One-Hot Encoding ✅

### 8. ML — Os 2 Modelos

| | Modelo 1 | Modelo 2 |
|---|:--------:|:--------:|
| Critério | **Gini** | **Entropy** |
| max_depth | **5** | **10** |

**Modelo 1 é melhor.** Modelo 2 sofre overfitting (gap treino-teste de 22pp).

### 9. Prata vs Ouro — Comparação

| Métrica | Prata | Ouro | Ganho |
|---------|:----:|:----:|:-----:|
| F1 | 45,7% | **51,4%** | **+5,7pp** |
| Acurácia | 51,3% | **56,4%** | **+5,1pp** |

**Por que melhorou?** One-Hot Encoding, IQR Clipping, efeito sinérgico.

### 10. Por que F1 é só 51%?

1. Correlação baixa (r < 0,25)
2. Poucos dados (778 linhas)
3. Árvore rasa (d=5) — proposital para evitar overfitting

**Para chegar a 70%:** mais dados, mais features, Random Forest/XGBoost.

### 11. Matriz de Confusão

```
               Previsto ALTO    Previsto BAIXO
Real ALTO      VP = 32 (20,5%)   FN = 46 (29,5%)
Real BAIXO     FP = 22 (14,1%)   VN = 56 (35,9%)
```

- Acurácia = 56,4%
- Precisão = 59,3% (quando diz "alto impacto", acerta 59%)
- Recall = 41,0% (só pega 41% dos reais)
- F1 = 48,5%

### 12. PySpark Refactoring

3 operações adicionadas:

**groupBy + agregação:** `count`, `mean`, `max` de `total_loss_usd` por `industry_primary`. **20 setores**, 0,57s.

**Window function:** `row_number()` particionando por `industry_primary`, ordenando por `total_loss_usd`. Top 3 por setor, 0,61s.

**Pandas vs PySpark JOIN:**

| Framework | Tempo | Linhas |
|-----------|:-----:|:------:|
| PySpark | **0,316s** | 778 |
| Pandas | **0,016s** | 778 |

PySpark é mais lento para 778 linhas (overhead JVM domina). Mas **escalaria para terabytes** — Pandas não.

### 13. Rust vs PySpark — Trade-offs

| Rust | PySpark |
|---|---|
| **0,15s** total | 6,22s total |
| Binário **52 MB portátil** (copia e roda) | Precisa JVM + Python + **1,9 GB** |
| Compila 2-3 min na 1ª vez | pip install 2-5 min |
| target/ 1,1 GB (cache de build) | .venv + Java = 1,9 GB |
| Ideal para produção batch | Ideal para exploração / big data |

### 14. Numba

Numba acelerou **fillna (6,48×)** e **zscore (3,27×)**, mas foi mais lento em **iqr (0,77×)** e **scaler (0,53×)**. Para 778 linhas, numpy vetorizado é difícil de superar. Numba brilha em datasets grandes (>100K).

---

---

## PARTE BENJAMIN — TUDO QUE OS OUTROS SABEM + EXTRAS ABAIXO

*Benjamin precisa saber tudo da parte comum **mais** os tópicos abaixo.*

---

### E1. Fit/Transform em profundidade

**Data leakage no pré-processamento:**
- Se calcular a mediana de `company_revenue_usd` com TODAS as 778 linhas e depois dividir treino/teste, a mediana já "viu" os valores de teste
- O modelo parece melhor do que realmente é
- **Correção:** FIT só no treino, TRANSFORM nos dois

**O que aconteceria sem fit/transform?**
- Métricas infladas em ~2-5pp
- Em produção (sem teste para "vazar"), desempenho real seria pior

### E2. Por que `direct_loss_usd` é o pior caso de leakage

`direct_loss_usd` + `indirect_loss_usd` ≈ `total_loss_usd`

Se mantivesse, o modelo aprenderia uma **conta aritmética**, não padrões de ataque. Teria acurácia de ~99% mas seria **inútil** para novos dados.

### E3. Por que `industry_primary` vira feature no Rust mas não no PySpark (Prata)

CSV tem `"51"`, `"52"`, `"31-33"`, `"44-45"` (códigos NAICS com hífens).

- **Rust (Polars):** `detectar_features()` tenta cast para Float64 — >50% são numéricos → coluna **incluída** (hífens viram NaN → 0.0)
- **PySpark (pandas):** `is_numeric_dtype()` retorna False → coluna **excluída**

**Impacto:** Rust tem 7 features na Prata vs 6 no PySpark.

### E4. Diferença de split Rust vs PySpark

- **Rust:** `df.slice(0, 622)` — primeiras 80% linhas (ordem do CSV)
- **PySpark:** `train_test_split(random_state=42)` — embaralhado

**Impacto:** Conjuntos treino/teste **diferentes** entre projetos → métricas não comparáveis. Cada projeto é consistente internamente.

### E5. Explicando os 52 MB do binário Rust

- **Compilação estática:** Polars, SmartCore, serde_json, chrono, uuid — tudo dentro do binário
- **Autossuficiente:** copia para qualquer Linux x86_64 e roda sem nada instalado
- `strip` reduz de 52 MB para ~5-8 MB
- `target/` com 3.052 arquivos — `cargo clean` recupera 1,1 GB

### E6. Possíveis perguntas difíceis na arguição

**Q: "Se Rust é 70× mais rápido, por que usar PySpark?"**
R: O texto.md exige refatoração para demonstrar escalabilidade. Rust é ideal para pipelines batch. PySpark escala para terabytes em clusters — o mesmo join em 100 TB faria em segundos, Pandas travaria.

**Q: "O pré-processamento melhorou o modelo? Quanto?"**
R: +5,7pp de F1 (45,7% → 51,4%). One-Hot + IQR Clipping + scaling.

**Q: "Por que F1 não passou de 60%?"**
R: Correlação baixa (r < 0,25) + dataset pequeno (778 linhas). Com 10.000+ e mais features, subiria.

**Q: "O que é data leakage e onde aplicou anti-leakage?"**
R: Informação do futuro vazando para o treino. Aplicado em 2 lugares: (1) remoção de colunas na Prata (`direct_loss_usd`, `disclosure_date`, etc.) e (2) fit/transform na Ouro.

**Q: "Por que mediana e não média como threshold?"**
R: Média (R$ 71M) é 4,3× a mediana (R$ 16,59M) por causa de outliers. Mediana divide em 50/50 (balanceado). Média daria ~80/20 (desbalanceado).

**Q: "Label Encoding vs One-Hot?"**
R: Ordinal → Label (confidence_tier: 4 > 3 > 2 > 1). Nominal → One-Hot (industry: Saúde ≠ Tecnologia). Inverter cria problemas: Label no nominal dá ordenação artificial; One-Hot no ordinal perde a ordem.

**Q: "Por que clipping e não remoção de outliers?"**
R: 93 outliers (12%) são os casos de alto impacto que o modelo tenta prever. Remover empobreceria. Clipping mantém o registro e a ordem relativa.

**Q: "Por que Numba não ajudou?"**
R: Numba acelera condicionais (fillna 6,48×, zscore 3,27×) mas perde para numpy em vetorizadas (iqr 0,77×, scaler 0,53×) em datasets pequenos. O gargalo do PySpark é a comunicação JVM↔Python, não processamento numérico.

---

## ROTEIRO DO NOTEBOOK — Demonstração ao Vivo

### Celula 1: Imports + SparkSession
```
Falar: "Inicializamos o Spark com 4GB de memória."
```
Mostrar Spark OK + versão.

### Celula 2-4: Bronze
```
Falar: "Bronze lê CSVs e converte para Parquet com metadados de auditoria."
```
Mostrar DataFrames com hash/timestamp.

### Celula 5-6: Prata
```
Falar: "INNER JOIN → 778 linhas. Removemos colunas com data leakage como direct_loss_usd."
```
**Destacar:** anti-leakage.

### Celula 7-9: EDA
```
Falar: "3 hipóteses, 6 gráficos. H2 refutada (Backdoor > Ransomware)."
```
Mostrar gráficos rapidamente.

### Celula 10: Ouro
```
Falar: "7 transformações com FIT/TRANSFORM. Parâmetros aprendidos só no treino."
```
Mostrar tabela de transformações. Destacar: 7 → 43 colunas.

### Celula 11-12: ML
```
Falar: "2 árvores. Gini d=5 (F1 48,5%) vence Entropy d=10 (overfitting)."
```
Mostrar métricas + matriz de confusão + comparação Prata vs Ouro.

### Celula 13-15: Refactoring PySpark
```
Falar: "3 operações: groupBy (20 setores), window (top 3 por setor), Pandas vs PySpark (0,02s vs 0,32s)."
```
**Concluir:** "PySpark é 20× mais lento aqui mas escala para terabytes."

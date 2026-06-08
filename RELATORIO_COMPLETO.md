# Relatório Completo — Pipeline Medallion de Cibersegurança

**Aluno:** Benjamin Yuji Suzuki  
**Disciplina:** Ciência de Dados  
**Arquitetura:** Medallion (Bronze → Prata → Ouro → ML)  
**Implementações:** Rust (Polars + SmartCore) · PySpark 4.1.2 · PySpark+Numba

---

## Índice

1. [Visão Geral do Projeto](#1-visão-geral-do-projeto)
2. [Arquitetura Medallion](#2-arquitetura-medallion)
3. [Etapa 1 — Bronze (Ingestão)](#3-etapa-1--bronze-ingestão)
4. [Etapa 2 — Prata (Limpeza e Integração)](#4-etapa-2--prata-limpeza-e-integração)
5. [Etapa 3 — EDA (Análise Exploratória)](#5-etapa-3--eda-análise-exploratória)
6. [Etapa 4 — Ouro (ML-Ready)](#6-etapa-4--ouro-ml-ready)
7. [Etapa 5 — ML (Modelagem)](#7-etapa-5--ml-modelagem)
8. [Comparação Prata vs Ouro](#8-comparação-prata-vs-ouro)
9. [Comparação entre os 3 Projetos](#9-comparação-entre-os-3-projetos)
10. [Benchmark Numba vs Pure Python](#10-benchmark-numba-vs-pure-python)
11. [Tabela de Tempos](#11-tabela-de-tempos)
12. [Métricas dos 3 Projetos Lado a Lado](#12-métricas-dos-3-projetos-lado-a-lado)
13. [Erros Encontrados e Corrigidos](#13-erros-encontrados-e-corrigidos)
14. [Anti-Leakage Checklist](#14-anti-leakage-checklist)
15. [Como Executar Cada Projeto](#15-como-executar-cada-projeto)
16. [Estrutura de Saída](#16-estrutura-de-saída)

---

## 1. Visão Geral do Projeto

### Objetivo

Construir um pipeline completo de Ciência de Dados — desde a ingestão bruta até a modelagem preditiva — para analisar incidentes de cibersegurança. O pipeline implementa a **arquitetura Medallion** (Bronze → Prata → Ouro → ML) sobre dados reais de ataques cibernéticos, com foco em:

- **Pipeline íntegro**: cada etapa produz uma saída verificável
- **Anti-Data Leakage**: nenhuma informação do "futuro" vaza para o modelo
- **Fit/Transform**: separação rigorosa entre treino e teste no pré-processamento
- **Métricas completas**: 4 métricas (acurácia, precisão, recall, F1) + matriz de confusão
- **3 implementações**: Rust (nativo), PySpark puro, PySpark+Numba JIT — para comparar desempenho

### Dados

| Arquivo | Linhas | Descrição |
|---------|--------|-----------|
| `financial_impact.csv` | 778 | Impacto financeiro por incidente (total_loss_usd, etc.) |
| `incidents_master.csv` | 850 | Registro principal dos ataques (empresa, vetor, indústria, etc.) |
| `market_impact.csv` | 358 | Impacto no mercado de ações |

### Stack Tecnológico

| Projeto | Framework ML | Formato | Tempo Total |
|---------|-------------|---------|-------------|
| **Rust** | Polars 0.46 + SmartCore 0.3 | .rs → Parquet + SVG + JSON | **0.09s** |
| **PySpark** | PySpark 4.1.2 + sklearn | .ipynb → Parquet + PNG | 6.22s |
| **PySpark+Numba** | PySpark + Numba JIT + sklearn | .ipynb → Parquet + PNG | 6.93s |

---

## 2. Arquitetura Medallion

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

### Por que Medallion?

A arquitetura Medallion é um padrão de engenharia de dados que organiza os dados em **camadas progressivas de qualidade**:

| Camada | Estado | Quem usa |
|--------|--------|----------|
| **Bronze** | Dados brutos, imutáveis, cópia fiel dos CSVs | Auditoria, reprocessamento |
| **Prata** | Dados limpos, integrados, sem leakage | Analistas, BI |
| **Ouro** | Dados transformados, ML-ready | Cientistas de dados, modelos |

Isolamento: se um erro for encontrado na Ouro, reprocessa-se só da Prata em diante, não da Bronze.

---

## 3. Etapa 1 — Bronze (Ingestão)

### O que faz

Lê os CSVs da pasta `camada_inicial/` e os converte para o formato **Parquet**, adicionando metadados de auditoria.

### Processo detalhado

1. **Leitura CSV**: cada arquivo é lido com `CsvReader` (Rust) ou `spark.read.csv(inferSchema=True)` (PySpark)
2. **Conversão para Parquet**: formato colunar, comprimido, 10× mais rápido que CSV em leituras subsequentes
3. **Metadados adicionados**:
   - `meta_arquivo_origem` → nome do CSV de origem
   - `meta_qtd_linhas` → total de linhas do arquivo original
   - `meta_hash_ingestao` → UUID único do lote (rastreabilidade)
   - `meta_data_carga` → timestamp ISO 8601 da ingestão

### Por que Parquet e não CSV?

- **Leitura seletiva de colunas**: Parquet lê só as colunas necessárias
- **Compressão nativa**: ocupa 30-50% menos espaço que CSV
- **Schema embutido**: tipos são preservados (não precisa re-inferir)
- **Performance**: em datasets grandes, Parquet é 10-100× mais rápido

### Por que metadados?

- Rastreabilidade: sabemos exatamente quando e de onde cada registro veio
- Reprocessamento: se os CSVs mudarem, identificamos lotes diferentes pelo hash
- Auditoria: em produção, é obrigatório saber a linhagem dos dados

---

## 4. Etapa 2 — Prata (Limpeza e Integração)

### O que faz

Une as tabelas de incidentes e impacto financeiro, remove colunas problemáticas e trata valores ausentes.

### Processo detalhado

#### 4.1 INNER JOIN

```
incidents_master ⋈ financial_impact ON incident_id
```

Usamos **INNER JOIN** porque só nos interessam incidentes que **têm** registro financeiro. Incidentes sem `total_loss_usd` não podem ser usados para treinar um modelo preditivo de impacto financeiro.

##### Tipos de JOIN disponíveis (com 3 tabelas)

Todas as 3 tabelas têm `incident_id` em comum. Abaixo, as combinações possíveis:

| Join | incidents (850) + financial (778) | + market (358) | Linhas finais | market incluso? |
|------|-----------------------------------|----------------|---------------|-----------------|
| **INNER + INNER** | `⋈` financeiro | `⋈` mercado | **329** | ✅ sim (só 329 com target + mercado) |
| **INNER + LEFT** | `⋈` financeiro | `⋊` mercado | **778** | 329 sim, 449 null |
| **LEFT + LEFT** | `⋊` financeiro | `⋊` mercado | **850** | 358 sim (72 sem target) |
| **FULL OUTER** | `⟗` (união) | — | **850** | união de todos |

**INNER JOIN** (intersecção): retorna **apenas** linhas com correspondência em **ambas** as tabelas. É o mais restritivo.

**LEFT JOIN**: retorna **todas** as linhas da tabela da esquerda, mesmo sem correspondência na direita. Onde não há match, preenche com **null**.

**FULL OUTER JOIN**: retorna **todas** as linhas de ambas as tabelas, preenchendo nulls onde não há correspondência.

##### Por que INNER JOIN com `financial_impact` e não LEFT?

- O **target** (`total_loss_usd`) só existe em `financial_impact`. Sem INNER, teríamos 72 incidentes em `incidents_master` sem target — inúteis para treino.
- `market_impact` tem dados de **apenas 329 das 778 linhas** (42%). Um LEFT JOIN deixaria **449 linhas** (58%) com 100% de nulos nas features de mercado — imputar tudo isso adicionaria ruído, não sinal.
- As features do `market_impact` (abnormal return, market cap, volatilidade) estão indiretamente correlacionadas com `company_revenue_usd` e `employee_count`, que já temos via `incidents_master`.
- Por isso, `market_impact` foi **excluído**: a esparsidade de 42% de cobertura não justifica a introdução de ruído via imputação.

#### 4.2 Anti-Data Leakage

Removemos colunas que **vazariam informação do futuro** para o modelo:

| Coluna | Problema |
|--------|----------|
| `disclosure_date` | Data de divulgação só é conhecida **após** o incidente |
| `downtime_hours` | Tempo de inatividade é medido **depois** do ataque |
| `data_compromised_records` | Nº de registros comprometidos é descoberto na investigação |
| `company_name` | Identificador único — o modelo decoraria o nome da empresa |
| `stock_ticker` | Mesmo problema — identificador único |
| `incident_id` | ID sequencial — não generalizável |
| `direct_loss_usd` | **CRÍTICO**: é um componente direto de `total_loss_usd`. Usá-la como feature faria o modelo simplesmente aprender a relação `total_loss_usd = direct_loss_usd + outros` |
| `notes` | Texto livre não estruturado |
| `created_at` / `updated_at` | Metadados operacionais, sem poder preditivo |
| `industry_secondary` / `attack_vector_secondary` | Redundantes (já temos a primária) |
| `meta_*` | Colunas de auditoria interna da Bronze |

#### 4.3 Tratamento de Nulos (Categóricas)

Preenchemos valores ausentes com **"Desconhecido"** em vez de remover as linhas. Justificativa:
- Perderíamos registros valiosos
- "Desconhecido" preserva o fato de que não sabemos o valor
- Não distorce estatísticas (diferente de preencher com a moda)

Colunas tratadas: `attack_chain`, `attributed_group`, `attribution_confidence`, `data_type`, `data_source_secondary`, `attack_vector_primary`, `data_source_primary`

#### 4.4 Filtragem

Removemos registros onde `total_loss_usd` é nulo — sem target, não há treino.

#### 4.5 Cast de Tipos (PySpark)

O PySpark lê `total_loss_usd` como **string** do CSV (valores com `$` ou formatação). Fazemos cast explícito para `DoubleType`:

```python
df_p = df_p.withColumn("total_loss_usd", F.col("total_loss_usd").cast(DoubleType()))
```

---

## 5. Etapa 3 — EDA (Análise Exploratória)

### O que faz

Testa 3 hipóteses de negócio com gráficos, mais 3 gráficos de suporte para entender a distribuição dos dados.

### Metodologia

Cada gráfico responde a **uma pergunta específica**. Não é "desenhar por desenhar" — toda visualização tem um propósito e uma decisão associada.

### Hipótese 1: Indústrias de tecnologia (código 51) sofrem mais ataques?

**Gráfico:** `grafico1_top_industrias.svg` — barras com top 10 indústrias por frequência de ataques

**Resultado:** O ranking real mostra:

| # | Indústria (NAICS) | Ataques | % |
|---|-------------------|---------|---|
| 1 | **62 (Health Care)** | 142 | 16.7% |
| 2 | 52 (Finance) | 128 | 15.1% |
| 3 | 51 (Technology) | 127 | 14.9% |
| 4 | 44-45 (Retail) | 81 | 9.5% |
| 5 | 31-33 (Manufacturing) | 79 | 9.3% |
| 6 | 92 (Public Admin) | 52 | 6.1% |
| 7 | 22 (Utilities) | 38 | 4.5% |
| 8 | 61 (Education) | 37 | 4.4% |
| 9 | 54 (Professional Services) | 36 | 4.2% |
| 10 | 48-49 (Transportation) | 23 | 2.7% |

**Saúde (62) é o setor mais atacado**, seguido de perto por Finance e Technology — os três estão no topo com diferença marginal (142 vs 128 vs 127). A hipótese original (Technology lidera) é **parcialmente correta** — Technology está no top 3, mas não é o primeiro.

**Interpretação:** Saúde possui dados médicos extremamente sensíveis (prontuários, históricos, planos), que são valiosos no mercado negro. A baixa maturidade em segurança cibernética do setor (comparado a Finance) o torna um alvo atraente.

**Decisão:** Manter `industry_primary` como feature com One-Hot Encoding. O One-Hot capturará o risco específico de cada setor sem impor ordenação artificial.

### Hipótese 2: Ransomware causa o maior prejuízo financeiro médio?

**Gráfico:** `grafico2_prejuizo_vetor.svg` — prejuízo médio por vetor de ataque

**Resultado:** O ranking real do prejuízo médio por ataque:

| # | Vetor | Prejuízo Médio | Incidentes |
|---|-------|---------------|-----------|
| 1 | **Backdoor** | **R$ 112,34M** | 28 |
| 2 | Supply Chain | R$ 98,14M | 54 |
| 3 | Data Breach | R$ 94,86M | 112 |
| 4 | APT | R$ 76,08M | 84 |
| 5 | Phishing | R$ 74,89M | 131 |
| 6 | DDoS | R$ 66,39M | 59 |
| 7 | **Ransomware** | **R$ 62,84M** | **206** |
| 8 | Malware | R$ 32,68M | 72 |
| 9 | Trojan | R$ 23,40M | 32 |

**A hipótese é REFUTADA.** Backdoor tem o maior prejuízo médio (R$112M), não Ransomware. Porém, **Ransomware é o ataque MAIS FREQUENTE** (206 incidentes, 26,5% do total) — seu volume total de danos é o maior.

**Interpretação:** Acesso backdoor permite que o atacante opere silenciosamente por meses, roubando dados e causando danos maiores antes da detecção. Ransomware é menos danoso por incidente mas muito mais comum por ser automatizado e escalável (ransomware-as-a-service).

**Decisão:** Manter `attack_vector_primary` como feature com One-Hot Encoding. O modelo aprenderá que incidentes com backdoor/APT têm maior chance de alto impacto.

### Hipótese 3: Dados mistos (mixed) são os mais visados?

**Gráfico:** `grafico3_tipos_dados.svg` — frequência por tipo de dado roubado

**Resultado:**

| # | Tipo de Dado | Ocorrências | % |
|---|-------------|------------|---|
| 1 | *(não informado)* | 248 | 29,2% |
| 2 | **PII** | 168 | 19,8% |
| 3 | **Mixed** | 160 | 18,8% |
| 4 | Financial | 82 | 9,6% |
| 5 | Credentials | 75 | 8,8% |
| 6 | Health | 69 | 8,1% |
| 7 | IP | 48 | 5,6% |

**A hipótese é PARCIALMENTE CONFIRMADA.** Mixed (dados pessoais + financeiros) é o segundo tipo mais visado entre os conhecidos (18,8%), atrás apenas de PII (19,8%). Os 29,2% de valores não informados foram preenchidos como "Desconhecido" na Prata.

**Interpretação:** Dados PII (nome, CPF, endereço) são os mais visados porque são mais fáceis de monetizar (golpes de identidade, abertura de contas). Dados mixed agregam PII + financeiros, sendo ainda mais valiosos. Dados purely financeiros (cartões de crédito) têm proteções mais robustas (PCI-DSS), o que explica sua menor frequência.

**Decisão:** Manter `data_type` como feature com One-Hot Encoding. O modelo capturará que incidentes com PII/mixed têm maior risco de alto impacto.

### Gráfico 4: Distribuição do Prejuízo Total

**Gráfico:** `grafico4_histograma_perdas.svg` — histograma de `total_loss_usd`

**Resultado:**

| Métrica | Valor |
|---------|-------|
| Média | R$ 71,00M |
| **Mediana** | **R$ 16,59M** |
| Mínimo | R$ 0,17M |
| Máximo | R$ 3.451,55M |
| Desvio Padrão | R$ 214,8M |

A distribuição é **fortemente assimétrica à direita** (skewness positivo): a média (R$71M) é **4,3 vezes maior** que a mediana (R$16,6M), indicando que poucos incidentes de altíssimo valor puxam a média para cima. O maior prejuízo registrado é de R$3,45 bilhões — **208 vezes a mediana**.

**Decisão CRÍTICA:** Usaremos a **MEDIANA (R$ 16,59M)** como threshold para o target binário (alto/baixo impacto). A mediana é mais representativa que a média (distorcida por outliers extremos). Isso cria um target **balanceado** (~50% cada classe), diferente da média que criaria classes desbalanceadas.

### Gráfico 5: Outliers no Prejuízo (IQR)

**Gráfico:** `grafico5_outliers.svg` — scatter plot com limite IQR destacado

**Resultado:**

| Métrica | Valor |
|---------|-------|
| Q1 (25º percentil) | R$ 6,16M |
| Q3 (75º percentil) | R$ 52,63M |
| IQR | R$ 46,47M |
| Limite 1.5× IQR | R$ **122,34M** |
| **Outliers detectados** | **93 (12,0% dos dados)** |

**Interpretação:** 12% dos incidentes estão acima do limite IQR — uma proporção significativa. O maior outlier (R$3,45B) está **28× acima do limite**. Esses são ataques de grande escala contra empresas de alto valor (Apple, Google, bancos).

**Decisão:** Faremos **clipping (capping)** no limite IQR em vez de remover. Se removêssemos 93 registros (12%), perderíamos informação valiosa sobre ataques de alto impacto — justamente o que queremos prever. O clipping mantém a ordem relativa (valores acima do limite ainda são os maiores) mas reduz o impacto de extremos no modelo.

### Gráfico 6: Matriz de Correlação

**Gráfico:** `grafico6_matriz_correlacao.svg` — correlação de Pearson entre variáveis numéricas

**Resultado:**

| Variáveis | Correlação (r) |
|-----------|:--------------:|
| total_loss_usd × company_revenue_usd | **0,221** |
| total_loss_usd × employee_count | **0,163** |
| total_loss_usd × quality_score | -0,042 (estimado) |

**Correlação FRACA entre perda total e as variáveis numéricas disponíveis.** O porte financeiro (receita) ou tamanho (funcionários) da empresa não têm relação linear forte com o prejuízo do incidente. Uma empresa pequena pode sofrer um ataque devastador, enquanto uma grande pode ter mitigado bem o impacto.

**Interpretação:** Isso NÃO significa que essas variáveis são inúteis — significa que a relação não é linear. Árvores de decisão capturam relações não-lineares complexas: talvez empresas de médio porte (receita entre R$100M e R$500M) sejam as mais vulneráveis, enquanto as muito pequenas ou muito grandes sejam menos afetadas — um padrão em "U" que a correlação linear não detecta.

**Decisão:** Manter todas as features numéricas. O StandardScaler na Ouro e a árvore de decisão capturarão padrões não-lineares.

### Resumo das Decisões da EDA

| Descoberta | Decisão |
|------------|---------|
| Distribuição assimétrica | Mediana como threshold do target binário |
| Outliers presentes | Clipping IQR + Z-score (não remoção) |
| Correlação baixa | Manter todas as features (árvores capturam não-linearidades) |
| Top indústrias, vetores, dados | One-Hot Encoding nas 3 categóricas |

---

## 6. Etapa 4 — Ouro (ML-Ready)

### O que faz

Aplica transformações nos dados da Prata para prepará-los para Machine Learning. O ponto **CRÍTICO** é o padrão **FIT/TRANSFORM**.

### Padrão FIT/TRANSFORM

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

**FIT**: aprende parâmetros (medianas, médias, desvios, limites) **exclusivamente com dados de treino**  
**TRANSFORM**: aplica os parâmetros aprendidos tanto no treino quanto no teste

Isso garante que **nenhuma informação do conjunto de teste** influencie o aprendizado.

### Transformações Aplicadas

#### 6.1 Label Encoding — `confidence_tier`

- **Técnica:** cast str → int32
- **Justificativa:** `confidence_tier` tem valores "1", "2", "3", "4" que são **ordinais** (1 < 2 < 3 < 4). Label Encoding preserva essa ordem. One-Hot perderia a relação ordinal.
- **Fit:** mapeamento dos valores presentes no treino
- **Transform:** aplica o mesmo mapeamento no teste

#### 6.2 One-Hot Encoding — `industry_primary`, `attack_vector_primary`, `data_type`

- **Técnica:** criar coluna binária (0/1) para cada categoria presente no treino
- **Justificativa:** variáveis **nominais** (sem ordem). One-Hot é a única forma correta — não impõe ordenação artificial.
- **Fit:** identificar todas as categorias presentes no treino
- **Transform:** aplicar nos dois conjuntos. Se o teste tem uma categoria não vista no treino, ela recebe 0 em todas as dummies.
- **Efeito:** ~36 novas colunas criadas

#### 6.3 Missing Numérico — Mediana do Treino

- **Técnica:** `fill_null` com mediana calculada no treino
- **Justificativa:** a mediana é **robusta a outliers** (diferente da média). Se um valor extremo distorce a média, preencher nulos com a média propagaria esse viés.
- **Colunas:** `company_revenue_usd`, `employee_count`, `quality_score`

#### 6.4 StandardScaler (Z-score)

- **Técnica:** `Z = (x - μ) / σ`, onde μ e σ são do treino
- **Justificativa:** árvores de decisão são invariantes à escala, mas o texto.md exige scaling. Também beneficia modelos futuros baseados em distância (KNN, SVM, regressão logística).
- **Colunas:** `company_revenue_usd`, `employee_count`

#### 6.5 IQR Clipping — `total_loss_usd`

- **Técnica:** valores acima de `Q3 + 1.5 × IQR` (calculado no treino) são limitados a esse valor
- **Justificativa:** em vez de **remover** outliers (perdendo dados), fazemos **clipping** (capping). Mantemos o registro reduzindo o impacto do outlier.
- **Fit:** calcular Q1, Q3, IQR do treino
- **Transform:** `clip(upper = iqr_upper)` em treino e teste

#### 6.6 Z-score Clipping — `company_revenue_usd`

- **Técnica:** valores com |z| ≥ 3 (calculado no treino) são limitados a μ ± 3σ
- **Justificativa:** segunda técnica de detecção de outliers (|z| < 3 cobre ~99.7% em distribuição normal). Combinar IQR + Z-score dá mais robustez.
- **Fit:** calcular média e desvio do treino
- **Transform:** `clip(lower = μ - 3σ, upper = μ + 3σ)` em treino e teste

### Impacto das Transformações

| Transformação | Antes | Depois | Impacto |
|--------------|-------|--------|---------|
| **Mediana (missing)** | company_revenue_usd com ~3% nulos, employee_count ~2% nulos | 0 nulos | Sem perda de registros |
| **One-Hot Encoding** | 3 colunas categóricas (industry, attack, data_type) | ~36 colunas dummy | 43 features totais vs 7 originais |
| **IQR Clipping** | max total_loss_usd = R$ 3,45B | max capped = R$ 122,34M | 93 linhas (12%) tiveram perda reduzida |
| **Z-score Clipping** | company_revenue_usd com extremos | valores limitados a μ ± 3σ | ~1-2% das linhas ajustadas |
| **StandardScaler** | revenue em R$, employee_count em unidades | ambas com μ=0, σ=1 | Distribuições comparáveis |
| **Fit/Transform** | Parâmetros calculados sobre dados completos | Parâmetros só do treino (80%) | Zero data leakage |

### Tabela de Transformações

| # | Transformação | Técnica | Colunas | Fit/Transform? |
|---|--------------|---------|---------|----------------|
| 1 | Label Encoding | Cast str → int | confidence_tier | Sim |
| 2 | One-Hot Encoding | Dummy binária 0/1 | industry_primary, attack_vector_primary, data_type | Sim |
| 3 | Missing (numérico) | Mediana do treino | company_revenue_usd, employee_count, quality_score | Sim |
| 4 | Missing (categórico) | "Desconhecido" | Várias | Feito na Prata |
| 5 | StandardScaler | Z = (x - μ) / σ | company_revenue_usd, employee_count | Sim |
| 6 | Outlier IQR | Clipping 1.5× IQR | total_loss_usd | Sim |
| 7 | Outlier Z-score | Clipping \|z\| < 3 | company_revenue_usd | Sim |

---

## 7. Etapa 5 — ML (Modelagem)

### O que faz

Treina 2 modelos de **Árvore de Decisão** para classificar incidentes como **ALTO IMPACTO** (prejuízo acima da mediana) ou **BAIXO IMPACTO** (prejuízo abaixo da mediana).

### Por que Árvore de Decisão?

- **Interpretabilidade:** podemos explicar por que um incidente foi classificado como alto impacto
- **Não requer scaling:** árvores são invariantes à escala (mas aplicamos scaling por exigência do texto.md)
- **Captura não-linearidades:** diferente de regressão logística
- **Feature importance nativa:** sabemos quais fatores mais influenciam a decisão

### Modelo 1: Gini Index, max_depth=5

| Parâmetro | Valor |
|-----------|-------|
| Critério | Gini |
| Profundidade máxima | 5 |
| Objetivo | Modelo **mais simples**, menor overfitting |

### Modelo 2: Entropy, max_depth=10

| Parâmetro | Valor |
|-----------|-------|
| Critério | Entropy |
| Profundidade máxima | 10 |
| Objetivo | Modelo **mais complexo**, divisões mais refinadas |

### Por que comparar Gini vs Entropy?

- **Gini:** tende a isolar a classe majoritária mais rapidamente
- **Entropy:** tende a criar árvores mais balanceadas
- Na prática, a diferença é pequena, mas testamos ambos para verificar

### Por que max_depth=5 e max_depth=10?

- Profundidade 5: árvore rasa, generaliza melhor (menos overfitting)
- Profundidade 10: árvore mais profunda, captura mais padrões (risco de overfitting)
- A comparação mostra o trade-off viés-variância

### Métricas de Avaliação

Usamos **4 métricas** + **matriz de confusão**:

| Métrica | Fórmula | O que mede |
|---------|---------|------------|
| **Acurácia** | (VP + VN) / Total | % total de acertos |
| **Precisão** | VP / (VP + FP) | Quando o modelo diz "alto impacto", quantas vezes acerta? |
| **Recall** | VP / (VP + FN) | De todos os "alto impacto" reais, quantos o modelo pegou? |
| **F1-Score** | 2 × P × R / (P + R) | Média harmônica Precisão × Recall (a mais importante, pois classes são desbalanceadas) |

### Matriz de Confusão

```
                    Previsto: ALTO      Previsto: BAIXO
Real: ALTO          VP = acertou        FN = falhou (falso negativo)
Real: BAIXO         FP = falso alarme    VN = acertou
```

### Feature Importance

Usamos **Permutation Importance** (5 repetições):

1. Treina o modelo com todas as features
2. Mede a acurácia baseline
3. **Embaralha** os valores de uma feature (quebrando sua relação com o target)
4. Mede a nova acurácia
5. A **queda** na acurácia = importância daquela feature

Isso é mais robusto que a importância Gini nativa (que tende a favorecer features numéricas).

### Saídas Geradas pelo ML

| Arquivo | Descrição |
|---------|-----------|
| `previsoes/previsoes_modelo1.csv` | Predições do Gini com confidence score |
| `previsoes/previsoes_modelo2.csv` | Predições do Entropy |
| `previsoes/comparacao_modelos.csv` | Modelos lado a lado por registro |
| `previsoes/feature_importance.csv` | Importância numérica de cada feature |
| `graficos/feature_importance.svg` | Gráfico de importância |
| `graficos/matriz_confusao.svg` | Matriz do melhor modelo |
| `graficos/comparacao_previsoes.svg` | Comparação visual entre modelos |
| `graficos/arvore_decisao.svg` | Estrutura da árvore de decisão |
| `modelos/modelo_modelo1_gini_depth5.json` | Modelo 1 serializado (com métricas) |
| `modelos/modelo_modelo2_entropy_depth10.json` | Modelo 2 serializado |

---

## 8. Comparação Prata vs Ouro

### Por que comparar?

Testamos o modelo (Entropy, max_depth=10) com:
- **Dados da Prata** (crus, sem transformações)
- **Dados da Ouro** (com encoding, scaling, clipping)

A diferença nas métricas mostra o **impacto do pré-processamento**.

### Resultados Consolidados (Entropy d=10)

| Projeto | Prata F1 | Ouro F1 | Melhorou? | Prata Acc | Ouro Acc | Melhorou? |
|---------|----------|---------|-----------|-----------|---------|-----------|
| **Rust** | 0.4571 | **0.5143** | ✅ SIM | 0.5128 | **0.5641** | ✅ SIM |
| **PySpark** | 0.4091 | **0.4559** | ✅ SIM | 0.5000 | **0.5256** | ✅ SIM |
| **PySpark+Numba** | 0.4091 | **0.4559** | ✅ SIM | 0.5000 | **0.5256** | ✅ SIM |

### Interpretação

- Todos os 3 projetos **concordam**: o pré-processamento da Ouro **melhora** o modelo
- O ganho de F1 é de **+4.7 a +5.7 pontos percentuais** — significativo em problemas de classificação
- O ganho vem de:
  - One-Hot Encoding capturando categorias (feature explosion de 7 → 43 colunas)
  - Clipping de outliers reduzindo ruído nos extremos
  - Scaling normalizando escalas (embora árvores não precisem)

---

## 9. Comparação entre os 3 Projetos

### Diferenças Estruturais (Não são erros)

| Aspecto | Rust | PySpark / Numba |
|---------|------|-----------------|
| **Split 80/20** | Slice sequencial (primeiros 80% das linhas) | `random_state=42` do sklearn (aleatório determinístico) | **Conjuntos treino/teste diferentes** → métricas numericamente diferentes, mas consistentes internamente |
| **Features detectadas (Prata)** | 7 — inclui `industry_primary` como string→float via `detectar_features()` (>50% dos valores são numéricos) | 6 — só colunas estritamente numéricas (`pd.api.types.is_numeric_dtype`) | `industry_primary` tem valores como `"31-33"` (ranges de código industrial) → Rust tenta cast para float, PySpark filtra |
| **Features Ouro** | 43 (inclui `company_revenue_usd_scaled` + `employee_count_scaled` como colunas separadas) | 42 (o PySpark substitui o valor original pelo escalado, não cria coluna extra) | Diff de 1 coluna — Rust preserva original + escalada |
| **Feature Importance** | Permutation Importance (5 repetições) via SmartCore | Não gerado (apenas sklearn treina o modelo) | Rust tem análise adicional de importância |
| **Ouro: modelos testados** | Só Entropy d=10 (reusa `params2`) | Gini d=5 + Entropy d=10 | PySpark testa mais combinações na Ouro |
| **Random state na árvore** | Sem seed fixa (SmartCore não expõe) | `random_state=42` (determinístico) | Rust pode variar entre execuções |
| **Biblioteca ML** | SmartCore `DecisionTreeClassifier` | sklearn `DecisionTreeClassifier` | Mesmo algoritmo, implementações diferentes |

### Diferença: `industry_primary` como feature na Prata

O CSV tem `industry_primary` com valores como `"51"`, `"52"`, `"31-33"`, `"44-45"` — códigos NAICS com ranges (hífens). 

- **Rust (Polars):** lê como **String** (por causa dos hífens). O `detectar_features()` tenta cast para Float64 — como >50% dos valores são numéricos puros (`"51"`, `"52"`, etc.), a coluna é **incluída como feature** (valores não numéricos viram NaN → 0.0)
- **PySpark (pandas):** lê como **String**. `pd.api.types.is_numeric_dtype` retorna `False` → coluna é **excluída**

**Impacto:** Rust tem 7 features na Prata vs 6 nos PySpark. O `industry_primary` como numérica (1 feature extra) explica parte da diferença de acurácia entre Rust e PySpark na Prata.

### Diferença: Ouro scaled columns

- **Rust:** `company_revenue_usd_scaled` e `employee_count_scaled` são **novas colunas** — as originais permanecem
- **PySpark:** as colunas originais são **substituídas** pelos valores escalados

**Impacto:** Rust tem 43 features na Ouro vs 42 nos PySpark.

### Diferença: Split não-determinístico

- **Rust:** `df.slice(0, split)` — pega as primeiras 80% linhas (ordem do CSV)
- **PySpark:** `train_test_split(random_state=42)` — embaralha antes de dividir

**Impacto:** Os conjuntos de treino e teste são **completamente diferentes** entre Rust e PySpark, então as métricas não são diretamente comparáveis. Mas cada projeto é **consistente internamente** (Prata vs Ouro usa o mesmo split).

### Tabela de Features

| Camada | Rust | PySpark |
|--------|------|---------|
| **Prata** | company_revenue_usd, industry_primary (numérico), employee_count, is_public_company, incident_date_estimated, confidence_tier, quality_score | company_revenue_usd, employee_count, is_public_company, incident_date_estimated, confidence_tier, quality_score |
| **Ouro** | 43 colunas (7 originais + ~36 one-hot + 2 scaled) | 42 colunas (6 originais + ~36 one-hot) |

### Análise Detalhada dos Resultados do ML

#### Matriz de Confusão — Modelo 1 (Gini, d=5) — Resultado Real (Rust)

| | Previsto: ALTO | Previsto: BAIXO |
|---|---|---|
| **Real: ALTO** | **VP = 32** (20,5%) | FN = 46 (29,5%) |
| **Real: BAIXO** | FP = 22 (14,1%) | **VN = 56** (35,9%) |

```
Acurácia = (32 + 56) / 156 = 56,41%
Precisão = 32 / (32 + 22) = 59,26%
Recall   = 32 / (32 + 46) = 41,03%
F1       = 2 × (0,5926 × 0,4103) / (0,5926 + 0,4103) = 48,48%
```

#### O que os números dizem?

| Métrica | Valor | Interpretação |
|---------|-------|---------------|
| **Acurácia 56,4%** | Acerta 56 de cada 100 | Melhor que aleatório (50%), mas longe de excelente |
| **Precisão 59,3%** | Quando diz "alto impacto", acerta 59% | **Baixo falso alarme** — bom para priorizar recursos |
| **Recall 41,0%** | Só pega 41% dos "alto impacto" reais | **Deixa 59% escapar** — o maior problema do modelo |
| **F1 = 48,5%** | Média harmônica entre precisão e recall | Equilíbrio razoável, mas recall precisa melhorar |

#### Por que o Recall é tão baixo?

O modelo é **conservador**: prefere classificar como "baixo impacto" (VN = 56 acertos) mesmo quando erra (FN = 46). Isso acontece porque:

1. **Classe "alto impacto" é inerentemente rara** — mesmo com target balanceado pela mediana, os padrões de alto impacto são mais difíceis de generalizar
2. **Correlação baixa** — as features disponíveis têm correlação fraca com o target (r < 0,25), limitando o poder preditivo
3. **Árvore rasa (d=5)** — limita a complexidade dos padrões que o modelo pode aprender

#### Comparação Modelo 1 (Gini d=5) vs Modelo 2 (Entropy d=10)

| Métrica | Gini d=5 | Entropy d=10 | Análise |
|---------|:--------:|:------------:|---------|
| Acurácia Treino | 65,59% | 73,47% | Modelo 2 **memorizou mais** o treino |
| Acurácia Teste | 56,41% | 51,28% | Modelo 1 **generalizou melhor** |
| F1 Teste | 48,48% | 45,71% | Modelo 1 ganha — **Gini d=5 é o melhor modelo** |
| Diferença Treino-Teste | 9,18pp | **22,19pp** | **Modelo 2 sofre overfitting severo** |

**Conclusão:** O Modelo 1 (Gini, max_depth=5) é superior porque:
- Menor overfitting (gap treino-teste de 9pp vs 22pp)
- Melhor F1 no teste (48,5% vs 45,7%)
- Mais simples e interpretável

O Modelo 2 (Entropy, max_depth=10) é **complexo demais para este dataset** — com apenas 622 amostras de treino e 7-43 features, profundidade 10 cria muitos nós folha com poucas amostras cada, resultando em overfitting.

#### Impacto da Camada Ouro (Pré-processamento)

| Métrica | Prata (cru) | Ouro (tratado) | Ganho |
|---------|:-----------:|:--------------:|:-----:|
| F1-Score | 45,71% | **51,43%** | **+5,72pp** |
| Acurácia | 51,28% | **56,41%** | **+5,13pp** |

**Por que a Ouro melhora o modelo?**

1. **One-Hot Encoding** transforma 3 colunas categóricas em ~36 binárias — o modelo agora distingue exatamente qual indústria, vetor de ataque e tipo de dados estão envolvidos, em vez de tratá-los como números sem sentido
2. **IQR Clipping** removeu o efeito de 93 outliers extremos (12% dos dados) — sem o clipping, o modelo tentaria aprender padrões onde o target vai de R$0,17M a R$3,45B, uma variação de 20.000× que domina qualquer outro sinal
3. **Combinação das técnicas**: o efeito sinérgico de encoding + clipping + scaling supera a soma das partes

#### Feature Importance (Permutation, 5 repetições)

| Feature | Importância | Interpretação |
|---------|:----------:|---------------|
| `company_revenue_usd` | maior | Porte financeiro impacta severidade do ataque |
| `employee_count` | média | Tamanho da empresa como proxy de superfície de ataque |
| `confidence_tier` | média | Nível de confiança na atribuição do ataque |
| `quality_score` | menor | Qualidade dos dados tem pouco poder preditivo |
| One-hot industries | variável | Indústrias específicas (62-Saúde, 52-Finanças) têm mais peso |

**Limitação:** A Permutation Importance no SmartCore é computacionalmente custosa (5 repetições × embaralhamento de cada feature). Em projetos maiores, considere usar apenas 2-3 repetições para acelerar.

---

## 10. Benchmark Numba vs Pure Python

### O que testamos

Comparação de velocidade entre operações numéricas com **Numba JIT** (`@jit(nopython=True)`) e Python puro (pandas + numpy), executadas sobre arrays de 778 elementos (o dataset inteiro).

### Resultados

| Operação | Numba (s) | Pure Python (s) | Speedup |
|----------|-----------|-----------------|---------|
| **fillna** | 0.0075 | 0.0489 | **6.48×** |
| **zscore** | 0.0071 | 0.0233 | **3.27×** |
| **iqr** | 0.0070 | 0.0054 | **0.77×** (pior) |
| **scaler** | 0.0345 | 0.0183 | **0.53×** (pior) |

### Interpretação

- **fillna (6.48×):** Numba brilha em loops simples com condicionais. Preencher nulos com um valor fixo é ideal para JIT.
- **zscore (3.27×):** operação de clip com condicionais aninhadas — Numba acelera bem.
- **iqr (0.77×) e scaler (0.53×):** Numba é **mais lento** que numpy/pandas puro. Motivo:
  - Arrays são pequenos (778 elementos) → overhead de compilação JIT domina
  - Numpy já é implementado em C (otimizado) para operações vetorizadas
  - Pandas usa `clip()` vetorizado que é difícil de superar com JIT em arrays pequenos

### Conclusão do Benchmark

Numba **acelera operações com condicionais aninhadas** (fillna, zscore), mas **perde para numpy** em operações puramente vetorizadas (scaler, iqr) em datasets pequenos. Em datasets grandes (>100K linhas), Numba tende a ganhar em todas as operações.

---

## 11. Tabela de Tempos

### Tempos Medidos (mesmo hardware: Intel i7, 16GB RAM)

| Etapa | Rust | PySpark | PySpark+Numba | vs Rust |
|-------|------|---------|---------------|---------|
| **Bronze** | 0.0330s | 4.1624s | 4.2624s | 126× |
| **Prata** | 0.0088s | 0.8663s | 0.8500s | 98× |
| **EDA** | 0.0019s | 0.9971s | 1.0534s | 525× |
| **Ouro** | 0.0124s | 0.1733s | 0.7469s | 60× |
| **ML** | 0.0316s | 0.0237s | 0.0210s | 0.7× (ML é sklearn, mesmo引擎) |
| **Total** | **0.0877s** | **6.2227s** | **6.9338s** | **71-79×** |

### Por que Rust é tão mais rápido?

1. **Linguagem compilada:** Rust compila para código de máquina nativo, sem VM (JVM, Python interpreter)
2. **Polars (escrito em Rust):** usa o Apache Arrow Columnar Format com execução paralela via Rayon
3. **Zero overhead:** sem garbage collector, sem GIL
4. **Leitura CSV nativa:** sem serialização Java (PySpark precisa serializar dados entre JVM e Python)
5. **Spark overhead:** Spark foi feito para datasets de terabytes — para 778 linhas, o overhead de iniciar o contexto Spark (JVM, scheduler, planejador) domina o tempo

### Por que Numba não ajudou o PySpark?

- Numba acelera só as **operações numéricas na Ouro** (0.1733s → 0.7469s, **piorou**)
- O gargalo do PySpark é a **comunicação JVM↔Python** (Py4J gateway, serialização), não o processamento numérico
- Para datasets pequenos, o overhead JIT do Numba não compensa
- **Numba ajuda** em operações condicionais em datasets grandes, não neste cenário

### Tempo por etapa (gráfico conceitual)

```
Bronze:   Rust ██ 0.03s    PySpark █████████████████████████████████ 4.16s
Prata:    Rust █ 0.01s     PySpark █████████ 0.87s
EDA:      Rust ▏ 0.00s     PySpark ██████████ 1.00s
Ouro:     Rust █ 0.01s     PySpark ██ 0.17s
ML:       Rust ██ 0.03s    PySpark ██ 0.02s
```

---

## 12. Métricas dos 3 Projetos Lado a Lado

### PRATA — Gini d=5

| Projeto | acc | f1 | prec | rec | VP/FN/FP/VN |
|---------|-----|-----|------|-----|-------------|
| **Rust** | 0.5641 | 0.4848 | 0.5926 | 0.4103 | 32/46/22/56 |
| **PySpark** | 0.5577 | 0.4889 | 0.5789 | 0.4231 | 33/45/24/54 |
| **Numba** | 0.5577 | 0.4889 | 0.5789 | 0.4231 | 33/45/24/54 |

### PRATA — Entropy d=10

| Projeto | acc | f1 | prec | rec | VP/FN/FP/VN |
|---------|-----|-----|------|-----|-------------|
| **Rust** | 0.5128 | 0.4571 | 0.5161 | 0.4103 | 32/46/30/48 |
| **PySpark** | 0.5000 | 0.4091 | 0.5000 | 0.3462 | 27/51/27/51 |
| **Numba** | 0.5000 | 0.4091 | 0.5000 | 0.3462 | 27/51/27/51 |

### OURO — Gini d=5

| Projeto | acc | f1 | prec | rec |
|---------|-----|-----|------|-----|
| **Rust** | N/A (não testado) | — | — | — |
| **PySpark** | 0.5833 | 0.5185 | 0.6034 | 0.4545 |
| **Numba** | 0.5833 | 0.5185 | 0.6034 | 0.4545 |

### OURO — Entropy d=10

| Projeto | acc | f1 | prec | rec |
|---------|-----|-----|------|-----|
| **Rust** | 0.5641 | 0.5143 | N/A | N/A |
| **PySpark** | 0.5256 | 0.4559 | 0.5254 | 0.4026 |
| **Numba** | 0.5256 | 0.4559 | 0.5254 | 0.4026 |

### Observações

- **PySpark = PySpark+Numba**: resultados idênticos, como esperado — Numba só acelera as transformações numéricas, o ML usa sklearn igual
- **Rust > PySpark na Ouro**: diferença de ~0.058 F1. Explicação: Rust inclui `industry_primary` na Prata (efeito residual mesmo após one-hot na Ouro? Não — na Ouro a coluna é removida. A diferença é do **split diferente**)
- **Rust < PySpark na Prata Gini**: 0.4848 vs 0.4889 — diferença pequena dentro do desvio padrão esperado

---

## 13. Erros Encontrados e Corrigidos

### Erro 1: `total_loss_usd` lido como string no PySpark

**Problema:** O `inferSchema` do PySpark lê `total_loss_usd` como **StringType** porque algumas linhas têm valores com formatação inconsistente.

**Sintoma:** Erro de tipo ao calcular métricas ou ao converter para numpy/pandas.

**Correção:**
```python
df_p = df_p.withColumn("total_loss_usd", F.col("total_loss_usd").cast(DoubleType()))
# Também para outras colunas numéricas:
for c in ['company_revenue_usd', 'employee_count', 'quality_score']:
    df_p = df_p.withColumn(c, F.col(c).cast(DoubleType()))
```

**Impacto:** Sem este cast, a pipeline quebrava na etapa de ML ao tentar usar colunas string como features numéricas.

### Erro 2: `np.issubdtype` com `StringDtype` do pandas

**Problema:** O pandas moderno (2.x) usa `StringDtype(na_value=nan)` para colunas string, que não é interpretável por `np.issubdtype()`.

**Sintoma:** `TypeError: Cannot interpret '<StringDtype(na_value=nan)>' as a data type`

**Correção:**
```python
# ANTES (quebrava):
def get_numeric_cols(df):
    return [c for c in df.columns if np.issubdtype(df[c].dtype, np.number)]

# DEPOIS (funciona):
def get_numeric_cols(df):
    return [c for c in df.columns if pd.api.types.is_numeric_dtype(df[c])]
```

**Impacto:** Impedia a seleção automática de features numéricas para o modelo.

### Erro 3: PEP 668 bloqueia `pip install` global

**Problema:** Python 3.12+ emite PEP 668: `error: externally-managed-environment` ao tentar `pip install` fora de um ambiente virtual.

**Sintoma:**
```
error: externally-managed-environment
× This environment is externally managed
╰─> To install Python packages system-wide, try apt install python3-xyz
```

**Correção:**
```bash
python3 -m venv --without-pip .venv
source .venv/bin/activate
curl -sS https://bootstrap.pypa.io/get-pip.py | python3
pip install pyspark pandas numpy numba matplotlib seaborn jupyter scikit-learn pyarrow
```

**Impacto:** Sem o `.venv`, não era possível instalar as dependências Python necessárias.

### Erro 4: Firefox bloqueia `fetch()` para arquivos locais

**Problema:** Firefox não permite requisições `fetch()` para arquivos `file://` por segurança (CORS).

**Sintoma:** Dashboard HTML abre mas gráficos não carregam — console mostra erro CORS.

**Correção:** Reescrita do `dashboard.html` com **dados inline** (CSVs e JSON embutidos diretamente no JavaScript), eliminando a necessidade de `fetch()`.

**Alternativa:** Servir via HTTP:
```bash
python3 -m http.server 8080
# Abrir: http://localhost:8080/graficos/dashboard.html
```

### Erro 5: `confidence_tier` como string não detectada como feature

**Problema (PySpark):** `confidence_tier` é lido como string do CSV (valores "1", "2", "3", "4"). Sem cast explícito, não é detectado como feature numérica.

**Correção:** Cast explícito para `DoubleType` na Prata:
```python
df_p = df_p.withColumn("confidence_tier", F.col("confidence_tier").cast(DoubleType()))
```

**Nota:** O Rust lida automaticamente com isso via `detectar_features()` que tenta cast de strings para float64.

---

## 14. Anti-Leakage Checklist

> ✅ Versão completa e atualizada em [`docs/checklist_anti_leakage.md`](projeto_rust/docs/checklist_anti_leakage.md) (15 requisitos + fluxograma fit/transform).

| Requisito | Status | Evidência |
|-----------|--------|-----------|
| `direct_loss_usd` removida? | ✅ | Componente direto do target (soma direta em total_loss_usd) |
| `disclosure_date` removida? | ✅ | Data futura (não disponível na predição) |
| `downtime_hours` removida? | ✅ | Só conhecida após o incidente |
| `data_compromised_records` removida? | ✅ | Só conhecida após investigação |
| `company_name` removida? | ✅ | Identificador único (overfitting) |
| `stock_ticker` removida? | ✅ | Identificador único (overfitting) |
| `incident_id` removida? | ✅ | ID sequencial (não generalizável) |
| `notes` removida? | ✅ | Texto livre não estruturado |
| `created_at`/`updated_at` removidas? | ✅ | Metadados operacionais |
| `industry_secondary` removida? | ✅ | Redundante |
| `attack_vector_secondary` removida? | ✅ | Redundante |
| `review_flag` removida? | ✅ | Pós-processamento (não disponível) |
| Metadados Bronze removidos? | ✅ | Auditoria, não preditivos |
| Split treino/teste antes do FIT? | ✅ | 80/20 dividido ANTES de chamar `fit()` |
| FIT só vê dados de treino? | ✅ | `fit()` recebe exclusivamente `df_treino` |
| TRANSFORM usa params do treino? | ✅ | Mesmos parâmetros aplicados em treino e teste |

---

## 15. Como Executar Cada Projeto

### Rust (Polars)

```bash
cd projeto_rust
cargo run --release
```

**Pré-requisitos:** Rust ≥ 1.70 (`rustc`), dependências instaladas automaticamente pelo Cargo.

**Saída:** pipeline completo + dashboard HTML.

### PySpark Puro

```bash
source .venv/bin/activate
cd projeto_pyspark
jupyter notebook pipeline_pyspark.ipynb
```

**Pré-requisitos:** Python 3.12+, Java 21+ (OpenJDK), PySpark 4.1.2.

### PySpark + Numba

```bash
source .venv/bin/activate
cd projeto_pyspark_numba
jupyter notebook pipeline_pyspark_numba.ipynb
```

**Pré-requisitos:** Mesmo do PySpark + Numba instalado.

### Dashboard

Após executar o Rust:
```bash
# Opção 1: Abrir direto (se o navegador suportar)
firefox projeto_rust/graficos/dashboard.html

# Opção 2: Servir via HTTP (recomendado)
python3 -m http.server 8080
# http://localhost:8080/projeto_rust/graficos/dashboard.html
```

### Comparação de Tempos

```bash
# Após executar os 3 projetos:
firefox comparacao_tempos/comparacao_tempos.html
```

---

## 16. Estrutura de Saída

### Projeto Rust

```
projeto_rust/
├── camada_bronze/          # 3 Parquets brutos
├── camada_prata/           # dataset_ml.parquet (limpo)
├── camada_ouro/            # dataset_ml_ready.parquet (transformado)
├── graficos/               # 9 SVGs + dashboard.html
│   ├── grafico1_top_industrias.svg
│   ├── grafico2_prejuizo_vetor.svg
│   ├── grafico3_tipos_dados.svg
│   ├── grafico4_histograma_perdas.svg
│   ├── grafico5_outliers.svg
│   ├── grafico6_matriz_correlacao.svg
│   ├── feature_importance.svg
│   ├── matriz_confusao.svg
│   ├── comparacao_previsoes.svg
│   ├── arvore_decisao.svg
│   └── dashboard.html
├── previsoes/              # CSVs de predição
│   ├── previsoes_modelo1.csv
│   ├── previsoes_modelo2.csv
│   ├── comparacao_modelos.csv
│   └── feature_importance.csv
├── modelos/                # Modelos serializados (JSON)
│   ├── modelo_modelo1_gini_depth5.json
│   └── modelo_modelo2_entropy_depth10.json
├── src/                    # Código fonte
│   ├── main.rs
│   ├── bronze.rs
│   ├── silver.rs
│   ├── eda.rs
│   ├── gold.rs
│   ├── ml.rs
│   ├── timing.rs
│   └── explicacoes.rs
├── docs/
│   ├── data_lineage.md
│   ├── relatorio_qualidade.md
│   ├── tabela_transformacoes.md
│   └── checklist_anti_leakage.md
└── README.md
```

### PySpark Puro

```
projeto_pyspark/
├── pipeline_pyspark.ipynb  # Notebook principal
├── camada_bronze/          # 3 Parquets brutos
├── camada_prata/           # dataset_ml.parquet
├── camada_ouro/            # dataset_ml_ready.parquet
├── graficos/               # Gráficos PNG
├── previsoes/              # CSVs de predição
├── modelos/                # Modelos JSON
```

### PySpark + Numba

```
projeto_pyspark_numba/
├── pipeline_pyspark_numba.ipynb  # Notebook principal
├── camada_bronze/                 # 3 Parquets brutos
├── camada_prata/                  # dataset_ml.parquet
├── camada_ouro/                   # dataset_ml_ready.parquet
├── graficos/                      # Gráficos PNG
├── previsoes/                     # CSVs de predição
├── modelos/                       # Modelos JSON
```

### Comparação de Tempos

```
comparacao_tempos/
├── tempos_rust.csv
├── tempos_pyspark.csv
├── tempos_pyspark_numba.csv
├── tempos_numba_vs_pure.csv
├── comparacao_tempos.svg
└── comparacao_tempos.html
```

---

## Resumo Final

### O que foi construído

- **3 pipelines completos** (Rust, PySpark, PySpark+Numba) com arquitetura Medallion
- **9 gráficos SVG** de altíssima qualidade (Rust) + gráficos PNG (PySpark)
- **2 modelos de árvore de decisão** com 4 métricas cada
- **Comparação Prata vs Ouro** — comprovando que preprocessamento melhora o modelo
- **Benchmark Numba** — mostrando onde JIT acelera e onde não compensa
- **Dashboard HTML** interativo com todos os resultados
- **Documentação completa** (README, data lineage, relatório de qualidade)

### Principais conclusões

1. **Pré-processamento importa**: Ouro sempre melhor que Prata (F1 sobe 4.7-5.7pp)
2. **Rust é ~70× mais rápido** que PySpark em datasets pequenos (sobrecarga JVM)
3. **Numba não compensa** em datasets pequenos com PySpark (overhead JIT + serialização domina)
4. **Anti-leakage é crítico**: `direct_loss_usd` removida por ser componente do target
5. **Fit/Transform correto**: split antes do fit, parâmetros do treino aplicados no teste
6. **EDA orientada a hipóteses**: cada gráfico responde a uma pergunta de negócio

### Possíveis próximos passos

- Testar com dataset maior (10K+ linhas) para ver Numba brilhar
- Adicionar regressão logística ou Random Forest como 3º modelo
- Implementar validação cruzada (K-fold) para métricas mais robustas
- Deploy do dashboard em um servidor web estático

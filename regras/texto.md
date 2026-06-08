# Projeto Prático — Camadas Ouro e ML-Ready

## Integração, Modelagem e Escalabilidade com PySpark

> Continuidade do pipeline de cibersegurança construído no 1º bimestre

---

## 📋 Visão Geral

Neste projeto, o grupo dará continuidade ao pipeline de engenharia de dados iniciado anteriormente, avançando da camada Prata para as camadas **Ouro** e **ML-Ready**, incorporando:

- Análise exploratória orientada a hipóteses
- Modelagem preditiva com Árvores de Decisão
- Refatoração com PySpark

---

## 📊 Etapa 1 — EDA Orientada a Hipóteses

### Requisitos

| # | Requisito | Descrição |
|---|-----------|-----------|
| 1 | **Mínimo 3 hipóteses** | Formular hipóteses sobre o comportamento dos incidentes de cibersegurança |
| 2 | **Mínimo 6 gráficos** | Cada hipótese deve ter ao menos 1 visualização que a sustente ou refute |
| 3 | **Distribuição de variáveis-chave** | Histograma, boxplot ou similar |
| 4 | **Análise de outliers** | IQR, Z-score ou similar |
| 5 | **Matriz de correlação** | Heatmap de correlação entre variáveis numéricas |
| 6 | **Análise por recorte** | Pelo menos 1: tipo de ataque, setor ou severidade |
| 7 | **Interpretação textual** | Cada gráfico deve ter conclusão **orientada a decisão** (não apenas descritiva) |

### Checklist de Entrega

- [ ] 3+ hipóteses formuladas
- [ ] 6+ gráficos gerados
- [ ] Distribuição de variáveis-chave
- [ ] Análise de outliers
- [ ] Matriz de correlação
- [ ] Análise por recorte (tipo de ataque/setor/severidade)
- [ ] Interpretações com conclusões orientadas a decisão

---

## 🏗️ Etapa 2 — Camada Ouro (ML-Ready)

### Requisitos

| # | Técnica | Especificação |
|---|---------|---------------|
| 1 | **Encoding 1** | Label Encoding (ordinal) |
| 2 | **Encoding 2** | One-Hot Encoding (nominal) |
| 3 | **Scaling** | 1 estratégia de scaling sobre variáveis numéricas relevantes |
| 4 | **Missing 1** | Estratégia para missing numérico |
| 5 | **Missing 2** | Estratégia para missing categórico |
| 6 | **Outlier 1** | Identificação e tratamento em coluna A |
| 7 | **Outlier 2** | Identificação e tratamento em coluna B |
| 8 | **Anti-leakage** | Remover colunas com risco de data leakage (ou justificar manutenção) |
| 9 | **Fit/Transform** | Pipeline deve seguir padrão fit/transform — **nada do conjunto de teste pode vazar para o treino** |
| 10 | **Parquet** | Dataset resultante salvo em Parquet |
| 11 | **Documentação** | Todas as transformações documentadas em uma tabela |

### Regras Críticas

```
Dataset completo
      │
      ├── 80% TREINO ──→ FIT (aprender parâmetros)
      │                        │
      │                        ↓
      ├── FIT params ──→ TRANSFORM no TREINO
      │
      └── 20% TESTE ──→ TRANSFORM com params do TREINO
```

> ⚠️ **FIT** aprende parâmetros exclusivamente com dados de treino.
> **TRANSFORM** aplica os parâmetros aprendidos tanto no treino quanto no teste.

### Checklist de Entrega

- [ ] 2+ técnicas de encoding diferentes
- [ ] 1+ estratégia de scaling
- [ ] 2+ estratégias de missing values
- [ ] Outliers tratados em 2+ colunas
- [ ] Anti-leakage aplicado
- [ ] Fit/Transform implementado corretamente
- [ ] Dataset salvo em Parquet
- [ ] Tabela de transformações gerada

---

## 🤖 Etapa 3 — Modelagem (ML)

### Requisitos

| # | Requisito | Especificação |
|---|-----------|---------------|
| 1 | **2+ modelos** | Árvores de Decisão com configurações distintas (ex: profundidade máxima, critério de divisão) |
| 2 | **Divisão treino/teste** | Justificada, com representatividade de classes em ambos os conjuntos |
| 3 | **3+ métricas** | Acurácia, precisão, recall, F1-score (mínimo 3) |
| 4 | **Matriz de confusão** | Para o melhor modelo treinado |
| 5 | **Visualização da árvore** | Estrutura da árvore de decisão resultante |
| 6 | **Comparação Prata vs Ouro** | **Ponto central**: comparar resultados com dados da Prata (cru) vs Ouro (tratado) |
| 7 | **Impacto do pré-processamento** | Demonstrar de forma clara o impacto na performance do modelo |

### Checklist de Entrega

- [ ] 2 modelos com configurações diferentes
- [ ] Divisão treino/teste justificada
- [ ] 3+ métricas calculadas
- [ ] Matriz de confusão gerada
- [ ] Árvore de decisão visualizada
- [ ] Comparação Prata vs Ouro realizada
- [ ] Impacto do pré-processamento demonstrado

---

## 🔄 Etapa 4 — Refatoração com PySpark

### Requisitos

| # | Requisito | Especificação |
|---|-----------|---------------|
| 1 | **2+ etapas refatoradas** | Substituir código original em Pandas por PySpark |
| 2 | **Leitura Parquet/Delta** | Ler dados em formato colunar |
| 3 | **Operação de JOIN** | Pelo menos 1 join |
| 4 | **groupBy + agregação** | Pelo menos 1 groupBy com agregação |
| 5 | **Função de janela** | Pelo menos 1 window function |
| 6 | **Escrita Parquet/Delta** | Salvar resultado em formato colunar |
| 7 | **Comparação de tempo** | Comparar tempo de execução Pandas vs PySpark em ao menos 1 etapa refatorada |
| 8 | **Discussão dos ganhos** | Analisar e discutir os ganhos observados |

### Checklist de Entrega

- [ ] 2+ etapas refatoradas para PySpark
- [ ] Leitura em Parquet/Delta
- [ ] Operação de JOIN
- [ ] groupBy com agregação
- [ ] Função de janela (Window)
- [ ] Escrita em Parquet/Delta
- [ ] Comparação de tempo Pandas vs PySpark
- [ ] Discussão dos ganhos observados

---

## 📦 Entregáveis

### Relação Completa

| # | Entregável | Formato | Obrigatório? |
|---|------------|---------|:------------:|
| 1 | Notebook principal com todas as etapas documentadas e executáveis | `.ipynb` | ✅ |
| 2 | Arquivos da camada Ouro | `.parquet` | ✅ |
| 3 | Relatório de qualidade atualizado (Prata + Ouro) | `.md` / `.pdf` | ✅ |
| 4 | Tabela de transformações da camada Ouro | `.md` / `.csv` | ✅ |
| 5 | Checklist anti-leakage atualizado | `.md` | ✅ |
| 6 | README com instruções de execução do pipeline completo | `.md` | ✅ |
| 7 | Data lineage atualizado (fluxo até camada Ouro) | `.md` / diagrama | ✅ |

---

## 🎤 Apresentação

### Formato

- **Duração:** 10 a 15 minutos por grupo
- **Estrutura obrigatória:**
  1. Visão geral do pipeline completo
  2. Demonstração ao vivo de ao menos 1 etapa executando no notebook
  3. Explicação das hipóteses e conclusões da EDA
  4. Justificativas das escolhas de pré-processamento
  5. Comparação dos resultados entre as camadas
  6. Demonstração da refatoração com PySpark

### Arguição Oral

- Após a apresentação, cada integrante responde individualmente a **1 pergunta sorteada pelo professor**
- A pergunta pode abordar **qualquer aspecto do projeto**:
  - Decisões de transformação
  - Conceitos de data leakage
  - Interpretação dos resultados do modelo
  - Uso do PySpark
- ⚠️ **Penalidade:** 0,5 ponto de desconto na nota da equipe **por pergunta não respondida corretamente**

---

## ✅ Pontuação

| Componente | Pontos |
|------------|:------:|
| Qualidade técnica dos entregáveis | **3,0** |
| Arguição oral | **2,0** |
| **Total** | **5,0** |

---

## 📋 Resumo dos Checklists

### Pipeline Completo

- [ ] EDA com 3+ hipóteses e 6+ gráficos
- [ ] Camada Ouro com encoding, scaling, missing, outliers, fit/transform
- [ ] 2 modelos de Árvore de Decisão com métricas
- [ ] Comparação Prata vs Ouro
- [ ] Refatoração PySpark com join, groupBy, window function
- [ ] Notebook executável
- [ ] Relatório de qualidade
- [ ] Tabela de transformações
- [ ] Checklist anti-leakage
- [ ] README
- [ ] Data lineage
- [ ] Apresentação de 10-15 min

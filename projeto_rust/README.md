# Pipeline Medallion — Análise de Incidentes de Cibersegurança

**Aluno:** Benjamin Yuji Suzuki  
**Disciplina:** Ciência de Dados  
**Linguagem:** Rust (Polars + SmartCore)  
**Arquitetura:** Medallion (Bronze → Prata → Ouro → ML)

---

## Requisitos

- **Rust** (edition 2021) — `rustc` >= 1.70
- Dependências Rust (instaladas automaticamente pelo `cargo`):
  - `polars` (0.46)
  - `smartcore` (0.3)
  - `chrono`, `uuid`, `serde_json`, `rand`

## Como Executar

```bash
cd projeto_rust
cargo run --release
```

O pipeline executa as 5 etapas em sequência:

1. **Bronze** — ingestão de CSVs para Parquet com metadados
2. **Prata** — join, anti-leakage, limpeza
3. **EDA** — 6 gráficos SVG com 3 hipóteses
4. **Ouro** — encoding, scaling, outliers, fit/transform
5. **ML** — 2 árvores de decisão, métricas, comparação

## Estrutura de Saída

| Pasta          | Conteúdo                                    |
|----------------|---------------------------------------------|
| `camada_bronze/` | 3 Parquets brutos com metadados           |
| `camada_prata/`  | Dataset limpo para ML                     |
| `camada_ouro/`   | Dataset ML-ready (transformado)           |
| `graficos/`      | 9 SVGs + dashboard.html                   |
| `previsoes/`     | CSVs de predição + importância            |
| `modelos/`       | Modelos serializados (JSON)               |

## Dashboard

Após executar o pipeline, abra o dashboard:

```
firefox graficos/dashboard.html
```

Ou via servidor HTTP:

```bash
python3 -m http.server 8080
# http://localhost:8080/graficos/dashboard.html
```

## Comparação de Performance (Rust vs PySpark vs PySpark+Numba)

Tempos de execução medidos no mesmo hardware (Intel i7, 16GB RAM):

| Etapa   | Rust    | PySpark | PySpark+Numba |
|---------|---------|---------|---------------|
| Bronze  | 0.054s  | 4.162s  | 4.262s        |
| Prata   | 0.009s  | 0.866s  | 0.850s        |
| EDA     | 0.003s  | 0.997s  | 1.053s        |
| Ouro    | 0.018s  | 0.173s  | 0.747s        |
| ML      | 0.034s  | 0.024s  | 0.021s        |
| **Total** | **0.117s** | **6.223s** | **6.934s**  |

Gráfico em `../comparacao_tempos/comparacao_tempos.html`.

> Rust é ~53× mais rápido que PySpark neste dataset pequeno (778 linhas).
> PySpark+Numba acelera operações numéricas (fillna: 6.5×, zscore: 3.3×)
> mas a sobrecarga JIT + serialização domina o tempo total.

## Projetos Relacionados

| Projeto | Descrição | Localização |
|---------|-----------|-------------|
| Rust (Polars) | Pipeline nativo ultra-rápido | `projeto_rust/` |
| PySpark puro | Pipeline com Spark DataFrame API | `projeto_pyspark/` |
| PySpark+Numba | Pipeline com Numba JIT para UDFs | `projeto_pyspark_numba/` |

## Data Lineage

Ver `docs/data_lineage.md` para o fluxo completo dos dados.

## Relatório de Qualidade

Ver `docs/relatorio_qualidade.md` para métricas de qualidade das camadas Prata e Ouro.

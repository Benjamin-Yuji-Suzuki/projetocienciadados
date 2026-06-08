# Checklist Anti-Leakage

## Objetivo

Garantir que **nenhuma informação do futuro (teste)** vaze para o
processo de treinamento do modelo. Cada item foi verificado no pipeline
e documentado com a respectiva justificativa.

## Checklist

| # | Requisito | Status | Onde | Justificativa |
|---|-----------|--------|------|---------------|
| 1 | `direct_loss_usd` removida? | ✅ | `silver.rs:40` | Componente direto de `total_loss_usd`. Se o modelo soubesse a perda direta, não precisaria prever a perda total |
| 2 | `disclosure_date` removida? | ✅ | `silver.rs:41` | Data de divulgação é posterior ao incidente — não está disponível no momento da predição |
| 3 | `downtime_hours` removida? | ✅ | `silver.rs:41` | Horas de inatividade só são conhecidas após o incidente ser resolvido |
| 4 | `company_name` removida? | ✅ | `silver.rs:42` | Identificador único da empresa. Modelo decoraria nomes em vez de generalizar |
| 5 | `stock_ticker` removida? | ✅ | `silver.rs:42` | Mesmo problema de `company_name` — identificador único |
| 6 | `incident_id` removida? | ✅ | `silver.rs:45` | ID sequencial — correlação espúria com severidade |
| 7 | Split treino/teste ANTES do fit? | ✅ | `gold.rs:289-292` | Divisão 80/20 realizada antes de qualquer cálculo de parâmetros |
| 8 | `fit()` só vê dados de TREINO? | ✅ | `gold.rs:297` | `fit(&df_treino)` recebe exclusivamente o conjunto de treino |
| 9 | `transform()` usa parâmetros do TREINO? | ✅ | `gold.rs:300-301` | Mesmo `TransformParams` aplicado em treino e teste |
| 10 | `notes` removida? | ✅ | `silver.rs:42` | Texto livre não estruturado poderia conter informações do resultado |
| 11 | `created_at` / `updated_at` removida? | ✅ | `silver.rs:42` | Metadados operacionais — data de criação/atualização não é preditiva |
| 12 | `data_compromised_records` removida? | ✅ | `silver.rs:41` | Nº de registros comprometidos só é conhecido após investigação |
| 13 | `review_flag` removida? | ✅ | `silver.rs:43` | Flag de revisão manual — indisponível em produção |
| 14 | `industry_secondary` / `attack_vector_secondary` removidas? | ✅ | `silver.rs:43` | Redundantes com as primárias e com muitos nulos |
| 15 | Metadados Bronze removidos? | ✅ | `silver.rs:44` | `meta_arquivo_origem`, `meta_qtd_linhas`, `meta_hash_ingestao`, `meta_data_carga` — só interessam para auditoria |

## Fluxo Fit/Transform (Diagrama)

```
Dados Completos (778 linhas)
       │
       ▼
  ┌──────────┐     ┌──────────┐
  │ TREINO   │     │ TESTE    │
  │ (80%)    │     │ (20%)    │
  │ 622 lin. │     │ 156 lin. │
  └────┬─────┘     └────┬─────┘
       │                │
       ▼                │
  ┌──────────┐          │
  │ FIT()    │──────────┼──→ Aprende: medianas, médias,
  │ params   │          │    desvios, limites IQR, limites Z
  └────┬─────┘          │
       │                │
       ▼                ▼
  ┌──────────┐     ┌──────────┐
  │TRANSFORM │     │TRANSFORM │
  │(params   │     │(MESMOS   │
  │ do treino)│     │ params)  │
  └────┬─────┘     └────┬─────┘
       │                │
       ▼                ▼
  ┌──────────┐     ┌──────────┐
  │ Treino   │     │ Teste    │
  │ tratado  │     │ tratado  │
  └──────────┘     └──────────┘
```

## Riscos Mitigados

| Risco | Como foi mitigado |
|-------|-------------------|
| **Target leakage** | `direct_loss_usd` removida (componente do target) |
| **Future data leakage** | `disclosure_date` removida (só conhecida após incidente) |
| **Identificador como feature** | `company_name`, `stock_ticker`, `incident_id` removidos |
| **Pós-incidente** | `downtime_hours`, `data_compromised_records` removidos |
| **Data snooping** | Split antes do fit, fit só vê treino, transform usa parâmetros do treino |
| **Metadados irrelevantes** | `notes`, `created_at`, `updated_at`, `review_flag` removidos |

## Conclusão

O pipeline foi verificado contra **15 requisitos anti-leakage**.
Todos foram atendidos. O modelo treinado não tem acesso a nenhuma
informação que não estaria disponível no momento da predição em
produção.

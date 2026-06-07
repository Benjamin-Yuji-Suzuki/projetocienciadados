#!/usr/bin/env python3
"""
Gera gráfico de comparação de tempos de execução
entre Rust, PySpark (puro) e PySpark + Numba.
"""
import csv
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
import numpy as np
import os

BASE = os.path.dirname(os.path.abspath(__file__))
DADOS = {
    'Rust':   os.path.join(BASE, 'tempos_rust.csv'),
    'PySpark': os.path.join(BASE, 'tempos_pyspark.csv'),
    'PySpark+Numba': os.path.join(BASE, 'tempos_pyspark_numba.csv'),
}

etapas = ['Bronze', 'Prata', 'EDA', 'Ouro', 'ML']
cores  = {'Rust': '#e94560', 'PySpark': '#457b9d', 'PySpark+Numba': '#2a9d8f'}
todos_tempos = {}
projeto_ordem = ['Rust', 'PySpark', 'PySpark+Numba']

for proj, path in DADOS.items():
    if not os.path.exists(path):
        print(f"[AVISO] {path} não encontrado. Pulando {proj}.")
        continue
    with open(path) as f:
        reader = csv.DictReader(f)
        tempos = {}
        for row in reader:
            if row['etapa'] in etapas:
                tempos[row['etapa']] = float(row['tempo_segundos'])
        todos_tempos[proj] = tempos

if not todos_tempos:
    print("Nenhum dado de tempo encontrado. Execute os pipelines primeiro.")
    exit(1)

x = np.arange(len(etapas))
w = 0.25

plt.figure(figsize=(14, 6))

for i, proj in enumerate(projeto_ordem):
    if proj not in todos_tempos:
        continue
    vals = [todos_tempos[proj].get(e, 0) for e in etapas]
    bars = plt.bar(x + i * w, vals, w, label=proj, color=cores[proj], alpha=0.85, edgecolor='white')
    for bar, v in zip(bars, vals):
        if v > 0:
            plt.text(bar.get_x() + bar.get_width()/2, bar.get_height() + max(vals)*0.01,
                     f'{v:.4f}s', ha='center', va='bottom', fontsize=8, rotation=45)

plt.xlabel('Etapa do Pipeline', fontsize=13)
plt.ylabel('Tempo (segundos)', fontsize=13)
plt.title('Comparação de Tempo de Execução: Rust vs PySpark vs PySpark+Numba', fontsize=15, fontweight='bold')
plt.xticks(x + w, etapas, fontsize=12)
plt.legend(fontsize=11)
plt.grid(axis='y', alpha=0.3)
plt.tight_layout()

out = os.path.join(BASE, 'comparacao_tempos.svg')
plt.savefig(out, dpi=150)
print(f"Gráfico salvo: {out}")

# Também gerar HTML com tabela
html = """<!DOCTYPE html>
<html lang="pt-BR">
<head><meta charset="UTF-8">
<title>Comparação de Tempos</title>
<style>
  body { font-family: 'Segoe UI', sans-serif; background: #0f0f23; color: #eee; padding: 40px; }
  h1 { color: #e94560; text-align: center; }
  table { width: 100%; border-collapse: collapse; margin: 20px auto; max-width: 900px; }
  th { background: #e94560; color: #fff; padding: 10px; }
  td { padding: 8px; border-bottom: 1px solid #333; text-align: center; }
  tr:hover { background: #1a1a3e; }
  .rust { color: #e94560; font-weight: bold; }
  .pyspark { color: #457b9d; font-weight: bold; }
  .numba { color: #2a9d8f; font-weight: bold; }
  img { display: block; margin: 30px auto; max-width: 1000px; border-radius: 12px; box-shadow: 0 0 30px rgba(233,69,96,0.2); }
</style></head>
<body>
<h1>⏱ Comparação de Tempos de Execução</h1>
<img src="comparacao_tempos.svg" alt="Gráfico de barras comparativo">
<table>
<thead><tr><th>Etapa</th>"""

for proj in projeto_ordem:
    if proj in todos_tempos:
        html += f"<th>{proj}</th>"

html += "</tr></thead><tbody>"

for etapa in etapas:
    cls_map = {'Rust': 'rust', 'PySpark': 'pyspark', 'PySpark+Numba': 'numba'}
    html += f"<tr><td><strong>{etapa}</strong></td>"
    for proj in projeto_ordem:
        if proj in todos_tempos:
            v = todos_tempos[proj].get(etapa, 0)
            html += f'<td class="{cls_map[proj]}">{v:.4f}s</td>'
    html += "</tr>"

html += "</tbody></table>"
html += '<p style="text-align:center;color:#888;">Gerado automaticamente pelo comparador de tempos</p>'
html += "</body></html>"

html_path = os.path.join(BASE, 'comparacao_tempos.html')
with open(html_path, 'w') as f:
    f.write(html)
print(f"HTML salvo: {html_path}")

#!/usr/bin/env bash
set -euo pipefail

VENV="/home/ben/Área de trabalho/Projeto Ciencia de dados/.venv"
PROJ_PYSPARK_NUMBA="/home/ben/Área de trabalho/Projeto Ciencia de dados/projeto_pyspark_numba"

echo "[setup] Ativando virtualenv..."
source "${VENV}/bin/activate"

echo "[setup] Instalando dependencias..."
pip install --quiet pyspark numba numpy pandas matplotlib seaborn scikit-learn jupyter

echo "[setup] Verificando Java (requisito PySpark)..."
if ! command -v java &>/dev/null; then
    echo "[WARNING] Java nao encontrado. PySpark requer Java 8+."
    echo "Instale com: sudo apt install openjdk-17-jdk"
fi

mkdir -p "${PROJ_PYSPARK_NUMBA}/camada_bronze"
mkdir -p "${PROJ_PYSPARK_NUMBA}/camada_prata"
mkdir -p "${PROJ_PYSPARK_NUMBA}/camada_ouro"
mkdir -p "${PROJ_PYSPARK_NUMBA}/graficos"
mkdir -p "${PROJ_PYSPARK_NUMBA}/modelos"
mkdir -p "${PROJ_PYSPARK_NUMBA}/previsoes"

CAMADA_INICIAL="/home/ben/Área de trabalho/Projeto Ciencia de dados/projeto_rust/camada_inicial"
if [ -d "$CAMADA_INICIAL" ]; then
    echo "[setup] Link simbolico para camada_inicial..."
    ln -sfn "$CAMADA_INICIAL" "${PROJ_PYSPARK_NUMBA}/camada_inicial"
fi

echo "[setup] Pronto! Execute: jupyter notebook pipeline_pyspark_numba.ipynb"

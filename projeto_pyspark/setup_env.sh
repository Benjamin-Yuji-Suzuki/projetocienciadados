#!/usr/bin/env bash
set -e

BASE_DIR="$(cd "$(dirname "$0")" && pwd)"

mkdir -p "$BASE_DIR/camada_bronze"
mkdir -p "$BASE_DIR/camada_prata"
mkdir -p "$BASE_DIR/camada_ouro"
mkdir -p "$BASE_DIR/graficos"
mkdir -p "$BASE_DIR/previsoes"
mkdir -p "$BASE_DIR/modelos"

echo "Diretorios criados em: $BASE_DIR"
echo "  - camada_bronze/"
echo "  - camada_prata/"
echo "  - camada_ouro/"
echo "  - graficos/"
echo "  - previsoes/"
echo "  - modelos/"

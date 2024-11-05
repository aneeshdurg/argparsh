#!/bin/bash
set -e

parser=$(python -m argparsh "$parser" add_arg "a" -- --choices "['a', 'b', 'c']")
parser=$(python -m argparsh "$parser" add_arg "-i" "--interval" -- --type int)
eval $(python -m argparsh "$parser" parse "$@")

echo "[bash]: A="$A
echo "[bash]: INTERVAL="$INTERVAL

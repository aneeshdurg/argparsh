parser=$(python -m argparsh "" init)
parser=$(python -m argparsh $parser add_arg "a")
parser=$(python -m argparsh $parser add_arg "-i" "--interval" -- --type int)
eval $(python -m argparsh $parser parse "$@")

echo "A="$A
echo "INTERVAL="$INTERVAL

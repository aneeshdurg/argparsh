#!/bin/bash
set -e

parser=$(python -m argparsh.new $0 -d "argparsh example" -e "bye!")
parser=$(python -m argparsh.add_arg $parser "a" -- \
  --choices "['a', 'b', 'c']"\
  --help "single letter arg")
parser=$(python -m argparsh.add_arg $parser "-i" "--interval" -- --type int)
parser=$(python -m argparsh.add_arg $parser "-f" -- --action store_true)

parser=$(python -m argparsh.subparser $parser init)
parser=$(python -m argparsh.subparser $parser create foo)
parser=$(python -m argparsh.subparser $parser create bar)

parser=$(python -m argparsh.add_arg --subparser foo $parser "qux")
parser=$(python -m argparsh.set_defaults --subparser foo $parser --myarg foo)

parser=$(python -m argparsh.add_arg --subparser bar $parser "baz")
parser=$(python -m argparsh.set_defaults --subparser bar $parser --myarg bar)

echo "Parsed args:"
python -m argparsh.parse $parser "$@"
eval $(python -m argparsh.parse $parser "$@")

echo "[bash]: A="$A
echo "[bash]: INTERVAL="$INTERVAL
echo "[bash]: F="$F
if [ $MYARG == "foo" ]; then
  echo "FOO: qux="$QUX
else
  echo "BAR: baz="$BAZ
fi

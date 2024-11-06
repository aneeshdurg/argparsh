#!/bin/bash
set -e

parser=$({
  argparsh new $0 -d "argparsh example" -e "bye!"
  argparsh add_arg "a" -- \
    --choices "['a', 'b', 'c']"\
    --help "single letter arg"
  argparsh add_arg "-i" "--interval" -- --type int
  argparsh add_arg "-f" -- --action store_true

  argparsh subparser_init --required true
  argparsh subparser_add foo
  argparsh subparser_add bar

  argparsh add_arg --subparser foo "qux"
  argparsh set_defaults --subparser foo --myarg foo

  argparsh add_arg --subparser bar "baz"
  argparsh set_defaults --subparser bar --myarg bar
})


echo "Parsed args:"
argparsh parse $parser "$@"
eval $(argparsh parse $parser "$@")

echo "[bash]: A="$A
echo "[bash]: INTERVAL="$INTERVAL
echo "[bash]: F="$F
if [ $MYARG == "foo" ]; then
  echo "FOO: qux="$QUX
else
  if [ $MYARG == "bar" ]; then
    echo "BAR: baz="$BAZ
  fi
fi

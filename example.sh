#!/bin/bash

# Create a parser program
parser=$({
  argparsh new $0 -d "argparsh example" -e "bye!"
  argparsh add_arg "a" -- \
    --choices "['a', 'b', 'c']"\
    --help "single letter arg"
  argparsh add_arg "-i" "--interval" -- --type int --default 10
  argparsh add_arg "-f" -- --action store_true

  argparsh subparser_init --required true
  argparsh subparser_add foo
  argparsh subparser_add bar

  argparsh add_arg --subparser foo "qux"
  argparsh set_defaults --subparser foo --myarg foo

  argparsh add_arg --subparser bar "baz"
  argparsh set_defaults --subparser bar --myarg bar
})

# Parse cli arguments as shell variables prefixed with "arg_"
#   cli arguments can be placed in the environment with "-e" or "--export"
#   cli arguments can be declared as local with "-l" or "--local"
eval $(argparsh parse $parser -p "arg_" -- "$@")

echo "Parsed args as shell variables:"
echo "[bash]: a="$arg_a
echo "[bash]: interval="$arg_interval
echo "[bash]: f="$arg_f
if [ "$arg_myarg" == "foo" ]; then
  echo "FOO: qux="$arg_qux
else
  if [ "$arg_myarg" == "bar" ]; then
    echo "BAR: baz="$arg_baz
  fi
fi

# Parse cli args into an associative array
eval $(argparsh parse $parser --format assoc_array --name args -- "$@")
echo "argument keys:" ${!args[@]}
echo "argument values:" ${args[@]}
echo "args['myarg'] =" ${args["myarg"]}

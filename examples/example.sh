#!/bin/bash

# Create a parser program
parser=$({
  argparsh new $0 -d "argparsh example" -e "bye!"
  argparsh add_arg \
    --choice a --choice b --choice c \
    --helptext "single letter arg" \
    -- "a"
  argparsh add_arg --type int --default 10 -- "-i" "--interval"
  argparsh add_arg --action store_true -- "-f"

  argparsh add_subparser foobar --required
  argparsh add_subcommand foo
  argparsh add_subcommand bar

  argparsh add_arg --subcommand foo "qux"
  argparsh add_arg --subcommand bar "baz"
})

# Parse cli arguments as shell variables prefixg ed with "arg_"
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
eval $(argparsh parse $parser --format assoc-array --name args -- "$@")
echo "argument keys:" ${!args[@]}
echo "argument values:" ${args[@]}
echo "args['foobar'] =" ${args["foobar"]}

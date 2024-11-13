#!/bin/bash

# Create a parser program
parser=$({
  argparsh new $0 -d "argparsh example" -e "bye!"
  argparsh add_arg "a" -- \
    --choices "['a', 'b', 'c']"\
    --help "single letter arg"
  argparsh add_arg "-i" "--interval" -- --type int --default 10
  argparsh add_arg "-f" -- --action store_true

  argparsh subparser foobar "foo or bar" --required true -- \
    foo --help "foo subcommand" -- $({
      argparsh add_arg "qux"
      argparsh set_defaults --myarg foo
    }) \
    bar --help "bar subcommand" -- $({
      argparsh add_arg "baz"
      argparsh set_defaults --myarg bar
    })
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

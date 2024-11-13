#!/usr/bin/env fish

# Create a parser program
set parser (
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
)


set args_json (argparsh parse $parser --format json -- "$@")
# In theory, one could extract the arguments from the json object here using jq,
# or some other utility. Another option is using the bass plugin to translate
# and execute the output with "--format shell".
# Of course, fish has an argparse utility built-in, and the point of this
# example is to show that argparsh can be run anywhere you can execute a child
# process, and doesn't rely on the shell for much.

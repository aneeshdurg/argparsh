#!/usr/bin/env fish

# Create a parser program
set parser (
  argparsh new (status filename) -d "argparsh example" -e "bye!"
  argparsh add_arg \
    -c a -c b -c c \
    --helptext "single letter arg" \
    -- "a"
  argparsh add_arg --type int --default 10 -- "-i" "--interval"
  argparsh add_arg --action store_true -- "-f"

  argparsh add_subparser foobar --required
  argparsh add_subcommand foo
  argparsh add_subcommand bar

  argparsh add_arg --subcommand foo "qux"
  argparsh add_arg --subcommand bar "baz"
)


set args_json (argparsh parse $parser --format json -- $argv)
# One could extract the arguments from the json object here using jq,
# or some other utility. Another option is using the bass plugin to translate
# and execute the output with "--format shell".
# Of course, fish has an argparse utility built-in, and the point of this
# example is to show that argparsh can be run anywhere you can execute a child
# process, and doesn't rely on the shell for much.
echo $args_json | jq
echo "foobar =" (echo $args_json | jq .foobar)

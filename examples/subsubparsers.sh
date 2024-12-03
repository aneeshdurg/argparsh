#!/bin/bash

parser=$({
  argparsh new $0

  argparsh add_subparser foobar --required
  argparsh add_subcommand foo
  argparsh add_subcommand bar

  argparsh add_subparser feefie --subcommand foo --required
  argparsh add_subcommand fee
  argparsh set_defaults --subcommand fee --myfooarg fee
  argparsh add_subcommand fie
  argparsh set_defaults --subcommand fie --myfooarg fie

  argparsh add_arg --subparserid foobar --subcommand foo "qux"
  argparsh set_defaults --subparserid foobar --subcommand foo --myarg foo

  argparsh add_arg --subparserid foobar --subcommand bar "baz"
  argparsh set_defaults --subparserid foobar --subcommand bar --myarg bar
})

argparsh parse $parser -- "$@"

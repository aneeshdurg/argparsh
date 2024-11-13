#!/bin/bash

parser=$({
  argparsh new $0

  argparsh subparser_init --metaname foobar --required true
  argparsh subparser_add foo
  argparsh subparser_add bar

  argparsh subparser_init --subparser foo --metaname feefie --required true
  argparsh subparser_add fee
  argparsh set_defaults --subparser fee --myfooarg fee
  argparsh subparser_add fie
  argparsh set_defaults --subparser fie --myfooarg fie

  argparsh add_arg --parser-arg foobar --subparser foo "qux"
  argparsh set_defaults --parser-arg foobar --subparser foo --myarg foo

  argparsh add_arg --parser-arg foobar --subparser bar "baz"
  argparsh set_defaults --parser-arg foobar --subparser bar --myarg bar
})

argparsh parse $parser -- "$@"

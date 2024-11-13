#!/bin/bash

parser=$({
  argparsh new $0

  # parser for the foo subcommand
  fooparser=$({
    argparsh set_defaults --myarg foo

    argparsh subparser "" "subsubcommand" -- \
      fee $(argparsh set_defaults --myfooarg fee) \
      fie $(argparsh set_defaults --myfooarg fie)

    argparsh add_arg "qux"
  })

  # parser for the bar subcommand
  barparser=$({
    argparsh set_defaults --myarg bar
    argparsh add_arg "baz"
  })

  # Register foo/bar subcommands on top-level parser
  argparsh subparser "foobar" "foo and bar toplvl subcommands" --required true -- \
    foo --help "foo subcommand" $fooparser \
    bar --help "bar subcommand" $barparser
})

argparsh parse $parser -- "$@"


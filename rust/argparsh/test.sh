parser=$({
  $PARSE new $0
  $PARSE add_arg -i --intval -- --required True --type int
  $PARSE add_arg -f --foo
  $PARSE add_arg -b --bar
  $PARSE add_arg --qux
  $PARSE add_arg --flux
})

$PARSE parse $parser -- "$@"

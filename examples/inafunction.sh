# You can even use argparsh inside of a single function - this is useful for
# small utilities that you might have inside a bashrc, for example

myutility() {
  local parser=$({
    argparsh new myutility -d "my favorite utility function"

    argparsh add_arg --helptext "source path" "src"
    argparsh add_arg --helptext "dest path" "dst"
  })

  # declare argument as local variables
  argparsh parse $parser --local -- "$@"

  mv $src $dst
}

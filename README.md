# argparsh - python argparse for the shell

Ever wanted to parse arguments in bash but felt frustrated by giant case blocks
and unfriendly syntax? Ever tried `getopts` but ended up curled on the floor
sobbing? Have you ever spent sleepless nights hoping that bash argument parsing
could be as simple as python's `argparse`? Maybe `argparsh` is for you.

`argparsh` aims to provide an easy way to construct an argument parsing program
from any shell.

## Usage

```bash
# Create a parser that accepts a string and an optional int value
parser=$({
    # Initialize the parser with the name of the script and a description
    argparsh new $0 -d "Example parser"

    # Add a positional argument - note that args after -- are options to add_arg
    # and not aliases for the argument
    argparsh add_arg strarg -- --help "My string argument"

    # Add a keyword argument that can either be -i <value> or --intarg <value>
    argparsh add_arg -i --intarg -- \
        --help "My int argument" \
        --type int \
        --default 10
})

# Parse the input arguments with the parser above
eval $(argparsh parse $parser -- "$@")

# Access parsed arguments by name
echo "String argument was" $strarg
echo "Integer argument was" $intarg
```

See `examples/example.sh` for a complete example, and examples of alternate
output formats (e.g. parsing CLI arguments into an associative array). The
`examples` directory has a few other examples that show different ways to use
`argparsh` in general.

## Installation

No dependencies besides python/pip.

```sh
pip install argparsh
```

Or, to build from source:

```sh
git clone https://github.com/aneeshdurg/argparsh.git
cd argparsh
pip install .
```

## Similar Works

argparsh differs from previous attempts at improving shell argument parsing by
being shell agnostic (although only bash is tested, it is relatively easy to add
support for more shells, contributions are welcome), supporting
subcommands/subparsers, and allowing detailed + customizable help text to be
configured.

+ [getopts](https://man7.org/linux/man-pages/man1/getopts.1p.html)
    - the OG argument handling utility. A bit clunky to use.
+ [argparse.sh](https://github.com/yaacov/argparse-sh)
    - similar approach, but not completely shell agnostic, and lacking in more
      advanced features, like subcommands.
+ [fish argparse](https://fishshell.com/docs/current/cmds/argparse.html)
    - Fish specific, but a nice utility. Not easy to use outside of fish and no
      subcommand support.

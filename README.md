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
        --default
})

# Parse the input arguments with the parser above
eval $(argparsh parse $parser "$@")

# Access parsed arguments by name
echo "String argument was" $STRARG
echo "Integer argument was" $INTARG
```

See `example.sh` for a more complete example.

### TODO

+ Subparsers
+ Support for more output formats (fish, JSON, ...)

## Installation

No dependencies besides python/pip.

```sh
pip install .
```

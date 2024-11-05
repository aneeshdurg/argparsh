# argparsh - python argparse for the shell

Ever wanted to parse arguments in bash but felt frustrated by giant case blocks
and unfriendly syntax? Ever tried `getopts` but ended up curled on the floor
sobbing? Have you ever spent sleepless nights hoping that bash argument parsing
could be as simple as python's `argparse`? Maybe `argparsh` is for you.

`argparsh` aims to provide an easy way to construct an argument parsing program
from any shell.

## Usage

```bash
# Create a state variable for the parser
parser=""
parser=${python -m argparsh $parser add_arg myarg)

# Parse the input arguments with the parser above
eval $(python -m argparsh $parser parse "$@")

# Access parsed arguments by name
echo $MYARG
```

### TODO

+ Subparsers
+ Support for more output formats (fish, JSON, ...)

## Installation

No dependencies besides python/pip.

```sh
pip install .
```

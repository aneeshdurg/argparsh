# argparsh

Ever wanted to parse arguments in bash but felt frustrated by giant case blocks
and unfriendly syntax? Ever tried `getopts` but ended up curled on the floor
sobbing? Have you ever spent sleepless nights hoping that bash argument parsing
could be as simple as python's
[argparse](https://docs.python.org/3/library/argparse.html)? Maybe `argparsh` is
for you.

`argparsh` aims to provide an easy way to construct an argument parsing program
from any shell. Unlike most other shell CLI parsing utilities, `argparsh`
supports advanced features such as subcommands and even provides multiple output
formats to allow maximum flexibility. `argparsh` reimplements the familiar,
well-documented `argparse` interface in pure Rust — no Python runtime required.

## Usage

```console
aneesh@earth:~/argparsh$ cat >> argparsh.sh
# Create a parser that accepts a string and an optional int value
parser=$({
    # Initialize the parser with the name of the script and a description
    argparsh new $0 -d "Example parser"

    # Add a positional argument - note that args after -- are options to add_arg
    # and not aliases for the argument
    argparsh add_arg --helptext "My string argument" strarg

    # Add a keyword argument that can either be -i <value> or --intarg <value>
    argparsh add_arg \
        --helptext "My int argument" \
        --type int \
        --default 10 \
        -- -i --intarg
})

# Parse the input arguments with the parser above
eval $(argparsh parse $parser -- "$@")

# Access parsed arguments by name
echo "String argument was" $strarg
echo "Integer argument was" $intarg
aneesh@earth:~/argparsh$ bash argparsh.sh -h
usage: argparsh.sh [-h] [-i INTARG] strarg

Example parser

positional arguments:
  strarg                My string argument

options:
  -h, --help            show this help message and exit
  -i INTARG, --intarg INTARG
                        My int argument
aneesh@earth:~/argparsh$ bash argparsh.sh -i na Hello
usage: argparsh.sh [-h] [-i INTARG] strarg
argparsh.sh: error: argument -i/--intarg: invalid int value: 'na'
aneesh@earth:~/argparsh$ bash argparsh.sh -i 10 Hello
String argument was Hello
Integer argument was 10
```

See `examples/example.sh` for a complete example, and examples of alternate
output formats (e.g. parsing CLI arguments into an associative array). The
`examples` directory has a few other examples that show different ways to use
`argparsh` in general.

More detailed help/documentation is available by running `argparsh <subcommand> --help`

## Installation

```sh
cargo install argparsh
```

Or, to build from source:

```sh
git clone https://github.com/aneeshdurg/argparsh.git
cd argparsh
cargo install --path .
```
Or, go to releases and download pre-built binaries.

### argparsh-nostd-demo

`nostd-demo/` contains `argparsh-nostd-demo`, a tiny `#![no_std]` binary
(about 70KB static, compared to about 1.6MB) that implements only the
parser-building commands (`new`, `add_arg`, `add_subparser`, `add_subcommand`,
`set_defaults`). Its output is byte-for-byte what `argparsh` emits, so you can
build the parser with it and still use `argparsh parse`. It does not
support `parse` and prints no help text. Invalid arguments still get a short
error and exit status 2.

```sh
cargo install --path nostd-demo
parser=$({
  argparsh-nostd-demo new prog
  argparsh-nostd-demo add_arg --type int -- -n
})
eval $(argparsh parse $parser -- "$@")
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

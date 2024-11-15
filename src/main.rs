use bitcode::{Decode, Encode};
use clap::{Parser, Subcommand, ValueEnum};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyTuple};

const DELIMITER: &str = "&";

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Encode, Decode)]
enum Format {
    /// Shell
    Shell,
    /// Associative Array
    AssocArray,
    /// json
    JSON,
}

impl ToString for Format {
    fn to_string(&self) -> String {
        match self {
            Format::Shell => "shell",
            Format::AssocArray => "assoc_array",
            Format::JSON => "json",
        }
        .to_string()
    }
}

const ADD_ARG_HELP: &str = r#"
Add an argument to the parser (separate argument aliases and parsing options with '--' ).
This is a wrapper around ArgumentParser.add_argument. In other words, the following invocation:
    argparsh add_arg [aliases]... -- [--key [value]]...
Is effectively:
    parser.add_argument(*[aliases], **[key/values])

argparsh is generally smart enough to parse and massage extra arguments into the correct types.
e.g.
    argparsh add_argument -i --intval -- --type int --default 10 --choices "[10, 20, 30]"

will become:
    parser.add_argument("-i", "--intval", type=int, default=100, choices=[10, 20, 30])

note: to add an argument for "-h" or "--help" one will need to run `argparsh -- -h ...`
note: to add an argument to a subparser use the --subparser and --parser-arg flags. These flags must
come before any aliases that are being registered. See the section on subparsers below for details.
"#;

const SUBPARSER_INIT_HELP: &str = r#"
Initialize a new subparser.
This is a wrapper around ArgumentParser.add_subparser, all keyword arguments are forwarded to
python.

The exceptions are:
    --metaname   The value provided to metaname
                 can be used to identify this subparser in
                 future calls to `add_arg` or `set_defaults`.
    --parser-arg This optional argument should be the metaname
                 of some previously created subparser. (See
                 below)
    --subparser  This optional argument should be the name of a
                 command attached to a previously created
                 subparser that we would like to create a new
                 subparser under. (See below)

e.g.
parser=$({
    # Create two subcommands `<prog> foo` and `<prog> bar`
    argparsh subparser_init --metaname foobar --required true
    argparsh subparser_add foo
    argparsh subparser_add bar

    # Attach a subcommand to `foo`, creating
    #    <prog> foo fee
    # -and-
    #    <prog> foo fie
    argparsh subparser_init --subparser foo --metaname feefie --required true
    argparsh subparser_add fee
    argparsh set_defaults --subparser fee --myfooarg fee
    argparsh subparser_add fie
    argparsh set_defaults --subparser fie --myfooarg fie

    # Add a regular argument to foo. Note that we now need to
    # use the metaname "foobar" so avoid attaching to the wrong
    # parser. (By default the most recently created parser is
    # used - in this case the most recently created parser is
    # feefie)
    argparsh add_arg --parser-arg foobar --subparser foo "qux"
    argparsh set_defaults --parser-arg foobar --subparser foo --myarg foo

    # Attach a regular argument to bar
    argparsh add_arg --parser-arg foobar --subparser bar "baz"
    argparsh set_defaults --parser-arg foobar --subparser bar --myarg bar

    # possible commands supported by this parser:
    #   <prog> foo fee <qux>
    #   <prog> foo fie <qux>
    #   <prog> bar <baz>
})
"#;

const SET_DEFAULTS_HELP: &str = r#"
Set defaults for parser with key/value pairs.

This is a wrapper around ArgumentParser.set_defaults, and is commonly used to attach default values
to a subparser to determine which subcommand was called. The subparser to attach to can be selected
using `--subparser` and `--parser-arg`. All other key/value pairs are forwarded.

e.g.:
    parser=$({
        argparsh subparser_init --metaname foo --required true

        argparsh subparser_add fee
        argparsh set_default --subparser fee --foocmd fee

        argparsh subparser_add fie
        argparsh set_default --subparser fee --foocmd fie
    })

    eval $(argparsh parse $parser -- "$@")
    echo "value for foo was: " $foocmd

If the above is called as `./prog.sh fee` it will print:
    value for foo was: fee
"#;

const PARSE_HELP: &str = r#"
Parse command line arguments

This command should usually be used with `eval` or some equivalent
to bring the parsed arguments into scope. e.g.:
    eval $(argparsh parse $parser -- "$@")

Note that `--` is used to separate arguments to `argparsh parse`
from the arguments being parsed.

Optionally, the `--format` option can be supplied to change the
output format.

--format shell [--prefix PREFIX] [-e/--export] [-l/--local]
    By default, the format is "shell", where every parsed argument
    is created as a shell varaible (with the syntax `KEY=VALUE`).
    Optionally, a prefix can be supplied with `--prefix` or `-p`:
        # Parse an argument named "value"
        parser=$(argparsh add_arg value)

        # Will create an variable named "arg_value"
        eval $(argparsh parse $parser -p arg_ -- "$@")
    the flags `--export`/`-e` and `--local`/`-l` will respectively
    either declare the variables as "export" (make the variable an
    environment variable) or "local" (bash/zsh only).

--format assoc_array --name NAME
    This declares a new associative array named `NAME` where every
    argument/value is a key/value entry in the associative array:
        # Parse an argument named "value"
        parser=$(argparsh add_arg value)

        # Will create a associative array (dictionary) variable named "args"
        eval $(argparsh parse $parser --format assoc_array --name args -- "$@")

        # Access the "value" key from $args
        echo ${args["value"]}

--format json
    outputs the parsed arguments as json

In any mode on failure to parse arguments for any reason (including
if the arguments invoked the help text), stdout will contain a
single line with the contents "exit <code>". And argparsh will exit
with the exit status also being set to `code`. Note that explit
invocation of help will result in a code of 0, while failure to
parse arguments will result in a non-zero code.
"#;

#[derive(Debug, Subcommand, PartialEq, Encode, Decode)]
enum Command {
    /// Create a new parser with a name and description
    New {
        /// Program Name
        name: String,
        /// Program description
        #[arg(short, long)]
        description: Option<String>,
        /// Help text epilog
        #[arg(short, long)]
        epilog: Option<String>,
    },
    /// Add argument to a parser or subparser
    #[command(name = "add_arg", long_about=ADD_ARG_HELP)]
    AddArg {
        /// Optional subparser command to add the argument to
        #[arg(long)]
        subparser: Option<String>,

        /// Optional parser metaname that is the parent of the command passed in with --subparser
        #[arg(long = "parser-arg")]
        parser_arg: Option<String>,

        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Option<Vec<String>>,
    },
    /// Initialize a new subparser
    #[command(name = "subparser_init", long_about=SUBPARSER_INIT_HELP)]
    SubparserInit {
        /// Optional subparser command to add the argument to
        #[arg(long)]
        subparser: Option<String>,

        /// Optional parser metaname that is the parent of the command passed in with --subparser
        #[arg(long = "parser-arg")]
        parser_arg: Option<String>,

        #[arg(long)]
        metaname: Option<String>,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Option<Vec<String>>,
    },
    /// Add a command to a subparser. See subparser_init for details
    #[command(name = "subparser_add")]
    SubparserAdd {
        /// Optional parser metaname that is the parent of the command passed in with --subparser
        #[arg(long)]
        metaname: Option<String>,
        /// Name of subcommand
        name: String,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Option<Vec<String>>,
    },
    /// Set default key/value pairs for a parser or subparser
    #[command(name = "set_defaults", long_about=SET_DEFAULTS_HELP)]
    SetDefaults {
        /// Optional subparser command to add the argument to
        #[arg(long)]
        subparser: Option<String>,

        /// Optional parser metaname that is the parent of the command passed in with --subparser
        #[arg(long = "parser-arg")]
        parser_arg: Option<String>,

        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Option<Vec<String>>,
    },
    /// Parse CLI args
    #[command(long_about=PARSE_HELP)]
    Parse {
        parser: String,

        #[arg(short, long, value_enum, default_value_t=Format::Shell)]
        format: Format,

        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Option<Vec<String>>,
    },
}

fn parse(parser: String, args: Option<Vec<String>>, format: Format) {
    let mut actions = parser.split(DELIMITER);
    actions.next();

    let py_res = Python::with_gil(|py| {
        let utils =
            PyModule::from_code_bound(py, include_str!("py/utils.py"), "utils.py", "utils")?;
        let parser = utils.getattr("Parser")?.call0()?;

        let add_arg = parser.getattr("cmd_add_argument")?;
        let add_subparser = parser.getattr("cmd_add_subparser")?;
        let add_subcommand = parser.getattr("cmd_add_subcommand")?;
        let set_defaults = parser.getattr("cmd_set_defaults")?;

        for act in actions {
            let cmd_json = urlencoding::decode_binary(act.as_bytes());
            let cmd: Command = bitcode::decode(&cmd_json).unwrap();
            match cmd {
                Command::New {
                    name,
                    description,
                    epilog,
                } => {
                    let parser_args = PyTuple::new_bound(py, vec![name]);

                    let parser_kwargs = PyDict::new_bound(py);
                    if let Some(v) = description {
                        parser_kwargs.set_item("description", v)?;
                    }
                    if let Some(v) = epilog {
                        parser_kwargs.set_item("epilog", v)?;
                    }

                    parser
                        .getattr("initialize")?
                        .call(parser_args, Some(&parser_kwargs))?;
                }
                Command::AddArg {
                    subparser,
                    parser_arg,
                    args,
                } => {
                    add_arg.call1((args, subparser, parser_arg))?;
                }
                Command::SubparserInit {
                    subparser,
                    parser_arg,
                    metaname,
                    args,
                } => {
                    add_subparser.call1((metaname, args, subparser, parser_arg))?;
                }
                Command::SubparserAdd {
                    metaname,
                    name,
                    args,
                } => {
                    add_subcommand.call1((metaname, name, args))?;
                }
                Command::SetDefaults {
                    subparser,
                    parser_arg,
                    args,
                } => {
                    set_defaults.call1((subparser, parser_arg, args))?;
                }
                Command::Parse { .. } => unreachable!(),
            }
        }

        utils
            .getattr("run_parse")?
            .call1((parser, format.to_string(), args))?;
        PyResult::Ok(0)
    });

    if let Err(ref x) = py_res {
        Python::with_gil(|py| {
            x.print_and_set_sys_last_vars(py);
        });
    }
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Parse {
            parser,
            format,
            args,
        } => {
            parse(parser, args, format);
        }
        _ => {
            let json = bitcode::encode(&cli.command);
            let s = urlencoding::encode_binary(&json);
            print!("{}{}", DELIMITER, s);
        }
    }
}

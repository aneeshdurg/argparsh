use bitcode::{Decode, Encode};
use clap::{Args, Parser, Subcommand, ValueEnum};
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

#[pyclass(eq, eq_int)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Encode, Decode)]
enum NArgs {
    /// '+' consumes at least one argument but possibly many
    #[clap(name = "+")]
    AtLeastOne,
    /// '*' consumes any number arguments
    #[clap(name = "*")]
    Many,
}

#[pyclass(eq, eq_int)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Encode, Decode)]
enum Action {
    /// Stores a single value
    Store,
    #[clap(name = "store_true")]
    /// Stores True (creates a boolean flag/niladic flags only)
    StoreTrue,
    /// Appends values together across multiple instances
    Append,
    /// Counts instances of the flag (niladic flags only)
    Count,
    /// Prints help text
    Help,
}

const ADD_ARG_HELP: &str = r#"
Add an argument to the parser (separate parsing options and aliases with '--' ).
This is a wrapper around ArgumentParser.add_argument. In other words, the following invocation:
    argparsh add_arg [OPTIONS] -- [aliases...]
Is effectively:
    parser.add_argument(*[aliases], **{key/values})

note: to add an argument for "-h" or "--help" one will need to run `argparsh -- -h ...`
note: to add an argument to a subparser use the --subparser and --parser-arg flags. These flags must
come before any aliases that are being registered. See the section on subparsers below for details.
"#;

const SUBPARSER_INIT_HELP: &str = r#"
Initialize a new subparser.
This is a wrapper around ArgumentParser.add_subparser, all keyword arguments are forwarded to
python.

The exceptions are:
    --parser-arg This optional argument should be the subparserid
                 of some previously created subparser. (See
                 below)
    --subparser  This optional argument should be the name of a
                 command attached to a previously created
                 subparser that we would like to create a new
                 subparser under. (See below)
Note that these two args MUST come before any others

e.g.
parser=$({
    # Create two subcommands `<prog> foo` and `<prog> bar`
    argparsh subparser_init foobar --required true
    argparsh subparser_add foo
    argparsh subparser_add bar

    # Attach a subcommand to `foo`, creating
    #    <prog> foo fee
    # -and-
    #    <prog> foo fie
    argparsh subparser_init feefie --subparser foo --required true
    argparsh subparser_add fee
    argparsh set_defaults --subparser fee --myfooarg fee
    argparsh subparser_add fie
    argparsh set_defaults --subparser fie --myfooarg fie

    # Add a regular argument to foo. Note that we now need to
    # use the subparserid "foobar" so avoid attaching to the wrong
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
        argparsh subparser_init --subparserid foo --required true

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

#[pyclass(get_all)]
#[derive(Debug, Args, PartialEq, Encode, Decode)]
struct AddArgCommand {
    /// Optional subparser command to add the argument to
    #[arg(long)]
    subparser: Option<String>,

    /// Optional parser subparserid that is the parent of the command passed in with --subparser
    #[arg(long = "parser-arg", requires = "subparser")]
    parser_arg: Option<String>,

    /// Number of arguments to consume (cannot be used with --nargs)
    #[arg(short, long)]
    nargs_exact: Option<usize>,

    #[arg(long, conflicts_with = "nargs_exact")]
    nargs: Option<NArgs>,

    /// The value produced if the argument is absent from the command line
    #[arg(short, long)]
    default: Option<String>,

    #[arg(short, long)]
    action: Option<Action>,

    /// Stores a constant value when the flag is passed (niladic flags only)
    #[arg(long, conflicts_with = "action")]
    store_const: Option<String>,

    /// Appends a constant value for each instance of the flag appearing (niladic flags only)
    #[arg(long, conflicts_with = "action", conflicts_with = "store_const")]
    append_const: Option<String>,

    /// Marks this argument as triggering version display
    #[arg(
        long,
        conflicts_with = "action",
        conflicts_with = "store_const",
        conflicts_with = "append_const",
        conflicts_with = "nargs",
        conflicts_with = "nargs_exact",
        requires = "version"
    )]
    displays_version: bool,

    /// Version format string to display when version argument is passed
    #[arg(long, requires = "displays_version")]
    version: Option<String>,

    /// Data type of argument (for validation only)
    #[arg(short, long, name = "type")]
    type_: Option<String>,

    /// Set choices for values (can be supplied multiple times)
    #[arg(short, long, action = clap::ArgAction::Append)]
    choice: Option<Vec<String>>,

    /// When supplied this argument will be marked as required
    #[arg(short, long, conflicts_with = "default")]
    required: bool,

    /// Help text for this argument
    #[arg(long)]
    helptext: Option<String>,

    /// Name to use for this variable in help text
    #[arg(long)]
    metavar: Option<String>,

    /// Destination variable name for this argument (default will be inferred from argument
    /// aliases)
    #[arg(long)]
    dest: Option<String>,

    /// Mark this argument as deprecated
    #[arg(long)]
    deprecated: bool,

    /// Optional argument name/flag and aliases. If omitted or if the values does not start with
    /// '-', then the argument will be treated as positional
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    args: Option<Vec<String>>,
}

#[pyclass(get_all)]
#[derive(Debug, Args, PartialEq, Encode, Decode)]
struct AddSubparserCommand {
    /// Optional parser subparserid that is the parent of the command passed in with --subparser
    /// This can be used to identify a specific subparser to attach arguments or defaults to
    /// later.
    #[arg(long)]
    subparserid: Option<String>,
    /// Name of subcommand
    name: String,

    /// name of the attribute under which sub-command name will be stored; by default the name will be used
    #[arg(short, long)]
    dest: Option<String>,

    /// Whether or not a subcommand must be provided, by default False
    #[arg(short, long)]
    required: bool,

    /// help text for sub-parser group in help output
    #[arg(long)]
    helptext: Option<String>,

    /// string presenting available subcommands in help; by default it is None and presents subcommands in form {cmd1, cmd2, ..}
    #[arg(short, long)]
    metavar: Option<String>,

    /// Optional subparser command to add the argument to (for sub-subparsers)
    #[arg(long)]
    subparser: Option<String>,

    /// Optional parser subparserid that is the parent of the command passed in with --subparser  (for sub-subparsers)
    #[arg(long = "parser-arg", requires = "subparser")]
    parser_arg: Option<String>,
}

#[pyclass(get_all)]
#[derive(Debug, Args, PartialEq, Encode, Decode)]
struct AddSubcommandCommand {
    /// Optional parser subparserid to add this command to
    #[arg(long)]
    subparserid: Option<String>,

    /// Name of subcommand
    name: String,

    /// help text for sub-parser group in help output
    #[arg(long)]
    helptext: Option<String>,
}

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
    AddArg(AddArgCommand),
    /// Initialize a new subparser
    #[command(name = "add_subparser", long_about=SUBPARSER_INIT_HELP)]
    AddSubparser(AddSubparserCommand),
    /// Add a command to a subparser. See subparser_init for details
    #[command(name = "add_subcommand")]
    AddSubcommand(AddSubcommandCommand),
    /// Set default key/value pairs for a parser or subparser
    #[command(name = "set_defaults", long_about=SET_DEFAULTS_HELP)]
    SetDefaults {
        /// Optional subparser command to add the argument to
        #[arg(long)]
        subparser: Option<String>,

        /// Optional parser subparserid that is the parent of the command passed in with --subparser
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
            PyModule::from_code_bound(py, include_str!("py/utils.py"), "argparsh", "utils")?;

        utils.add_class::<AddArgCommand>()?;
        utils.add_class::<Action>()?;
        utils.add_class::<NArgs>()?;

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
                Command::AddArg(opts) => {
                    add_arg.call1((opts,))?;
                }
                Command::AddSubparser(opts) => {
                    add_subparser.call1((opts,))?;
                }
                Command::AddSubcommand(opts) => {
                    add_subcommand.call1((opts,))?;
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

use bitcode::{Decode, Encode};
use clap::{Args, Parser, Subcommand, ValueEnum};
use std::vec::Vec;

mod argparse;

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

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Encode, Decode)]
enum NArgs {
    /// '+' consumes at least one argument but possibly many
    #[clap(name = "+")]
    AtLeastOne,
    /// '*' consumes any number arguments
    #[clap(name = "*")]
    Many,
}

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

#[derive(Debug, Args, PartialEq, Encode, Decode)]
struct AddArgCommand {
    /// Optional subparser command to add the argument to
    #[arg(long)]
    subcommand: Option<String>,

    /// Optional parser subparserid that is the parent of the command passed in with --subcommand
    #[arg(long, requires = "subcommand")]
    subparserid: Option<String>,

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

#[derive(Debug, Args, PartialEq, Encode, Decode)]
struct AddSubparserCommand {
    /// Optional parser subparserid that is the parent of the command passed in with --subcommand
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
    subcommand: Option<String>,

    /// Optional parser subparserid that is the parent of the command passed in with --subcommand  (for sub-subparsers)
    #[arg(long, requires = "subcommand")]
    parent_subparserid: Option<String>,
}

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

const ADD_ARG_HELP: &str = r#"
Add an argument to the parser (separate parsing options and aliases with '--' ).
This is a wrapper around ArgumentParser.add_argument. In other words, the following invocation:
    argparsh add_arg [OPTIONS] -- [aliases...]
Is effectively:
    parser.add_argument(*[aliases], **{key/values})

note: to add an argument for "-h" or "--help" one will need to run `argparsh -- -h ...`
note: to add an argument to a subparser use the --subcommand and --subparserid flags. These flags must
come before any aliases that are being registered. See the section on subparsers below for details.
"#;

const ADD_SUBPARSER_HELP: &str = r#"
Initialize a new subparser.
This is a wrapper around ArgumentParser.add_subparsers.

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
    argparsh subparser_init feefie --subcommand foo --required true
    argparsh subparser_add fee
    argparsh set_defaults --subcommand fee --myfooarg fee
    argparsh subparser_add fie
    argparsh set_defaults --subcommand fie --myfooarg fie

    # Add a regular argument to foo. Note that we now need to
    # use the subparserid "foobar" so avoid attaching to the wrong
    # parser. (By default the most recently created parser is
    # used - in this case the most recently created parser is
    # feefie)
    argparsh add_arg --subparserid foobar --subcommand foo "qux"
    argparsh set_defaults --subparserid foobar --subcommand foo --myarg foo

    # Attach a regular argument to bar
    argparsh add_arg --subparserid foobar --subcommand bar "baz"
    argparsh set_defaults --subparserid foobar --subcommand bar --myarg bar

    # possible commands supported by this parser:
    #   <prog> foo fee <qux>
    #   <prog> foo fie <qux>
    #   <prog> bar <baz>
})
"#;

const SET_DEFAULTS_HELP: &str = r#"
Set defaults for parser with key/value pairs.

This is a wrapper around ArgumentParser.set_defaults. The subparser to attach to can be selected
using `--subcommand` and `--subparserid`. All other key/value pairs are forwarded.

e.g.:
    parser=$({
        argparsh subparser_init --subparserid foo --required true

        argparsh subparser_add fee
        argparsh set_default --subcommand fee --foocmd fee

        argparsh subparser_add fie
        argparsh set_default --subcommand fee --foocmd fie
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
    AddArg(AddArgCommand),
    /// Initialize a new subparser
    #[command(name = "add_subparser", long_about=ADD_SUBPARSER_HELP)]
    AddSubparser(AddSubparserCommand),
    /// Add a command to a subparser. See subparser_init for details
    #[command(name = "add_subcommand")]
    AddSubcommand(AddSubcommandCommand),
    /// Set default key/value pairs for a parser or subparser
    #[command(name = "set_defaults", long_about=SET_DEFAULTS_HELP)]
    SetDefaults {
        /// Optional subcommand to add the argument to
        #[arg(long)]
        subcommand: Option<String>,

        /// Optional subparserid that is the parent of the command passed in with --subcommand
        #[arg(long)]
        subparserid: Option<String>,

        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Option<Vec<String>>,
    },
    /// Parse CLI args
    #[command(long_about=PARSE_HELP)]
    Parse {
        parser: String,

        #[arg(short, long, value_enum, default_value_t=Format::Shell)]
        format: Format,

        /// Prefix to add to every declared variable (shell format only)
        #[arg(short, long)]
        prefix: Option<String>,

        /// Export declarations to the environment (shell format only)
        #[arg(short, long)]
        export: bool,

        /// Declare variable as local (shell format only)
        #[arg(short, long)]
        local: bool,

        /// Name of variable to output into (assoc_array format only)
        #[arg(short, long)]
        name: Option<String>,

        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Option<Vec<String>>,
    },
}

fn parse(
    parser: String,
    args: Option<Vec<String>>,
    format: Format,
    prefix: Option<String>,
    export: bool,
    local: bool,
    name: Option<String>,
) {
    let mut actions = parser.split(DELIMITER);
    actions.next();

    let mut parser_model = argparse::Parser::new();
    for act in actions {
        let cmd_json = urlencoding::decode_binary(act.as_bytes());
        let cmd: Command = bitcode::decode(&cmd_json).unwrap();
        match cmd {
            Command::New {
                name,
                description,
                epilog,
            } => {
                parser_model.initialize(name, description, epilog);
            }
            Command::AddArg(opts) => {
                parser_model.add_argument(opts);
            }
            Command::AddSubparser(opts) => {
                parser_model.add_subparser(opts);
            }
            Command::AddSubcommand(opts) => {
                parser_model.add_subcommand(opts);
            }
            Command::SetDefaults {
                subcommand,
                subparserid,
                args,
            } => {
                parser_model.set_defaults(subcommand, subparserid, args);
            }
            Command::Parse { .. } => unreachable!(),
        }
    }

    let input_args = args.unwrap_or_default();
    // Split into extra_args (before --) and args (after --)
    let mut extra_args = Vec::new();
    let mut found_sep = false;
    let mut remaining = input_args;
    while let Some(first) = remaining.first() {
        if first == "--" {
            remaining.remove(0);
            found_sep = true;
            break;
        }
        extra_args.push(remaining.remove(0));
    }
    if !found_sep {
        remaining = extra_args.clone();
        extra_args.clear();
    }

    match parser_model.parse_args(remaining) {
        argparse::ParseResult::Success(kv) => match format {
            Format::JSON => {
                let mut json_kv = serde_json::Map::new();
                for (k, v) in &kv {
                    json_kv.insert(k.clone(), v.clone().into());
                }
                let value = serde_json::Value::Object(json_kv);
                let json = serde_json::to_string_pretty(&value).unwrap();
                println!("{}", json);
            }
            Format::Shell => {
                let prefix_str = prefix.unwrap_or_default();
                let export_str = if export {
                    "export "
                } else if local {
                    "local "
                } else {
                    ""
                };
                for (k, v) in &kv {
                    println!("{}{}{}={}", export_str, prefix_str, k, format_value(v));
                }
            }
            Format::AssocArray => {
                let name_str = name.unwrap_or_default();
                println!("declare -A {}", name_str);
                for (k, v) in &kv {
                    println!("{}[\"{}\"]={}", name_str, k, format_value(v));
                }
            }
        },
        argparse::ParseResult::Help(help_text) => {
            if std::env::var("ARGPARSH_DEBUG_HELP").is_ok() {
                use std::io::Write;
                std::fs::File::create("/tmp/rust_help.txt")
                    .unwrap()
                    .write_all(help_text.as_bytes())
                    .unwrap();
            }
            eprint!("{}", help_text);
            println!("exit 0");
            std::process::exit(0);
        }
        argparse::ParseResult::Version(v) => {
            eprintln!("{}", v);
            println!("exit 0");
            std::process::exit(0);
        }
        argparse::ParseResult::Error(err) => {
            eprintln!("{}\n{}", parser_model.usage_line(), err);
            println!("exit 2");
            std::process::exit(2);
        }
    }
}

fn format_value(v: &argparse::ArgValue) -> String {
    match v {
        argparse::ArgValue::String(s) => s.clone(),
        argparse::ArgValue::Int(i) => i.to_string(),
        argparse::ArgValue::Float(f) => f.to_string(),
        argparse::ArgValue::Bool(b) => b.to_string(),
        argparse::ArgValue::Null => "null".to_string(),
        argparse::ArgValue::List(lst) => lst
            .iter()
            .map(|x| format_value(x))
            .collect::<Vec<_>>()
            .join(" "),
    }
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Parse {
            parser,
            format,
            prefix,
            export,
            local,
            name,
            args,
        } => {
            parse(parser, args, format, prefix, export, local, name);
        }
        _ => {
            let json = bitcode::encode(&cli.command);
            let s = urlencoding::encode_binary(&json);
            print!("{}{}", DELIMITER, s);
        }
    }
}

use clap::{Parser, Subcommand, ValueEnum};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyTuple};
use serde::{Deserialize, Serialize};

extern crate serde_qs as qs;

const DELIMITER: &str = "&argparsh";

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Serialize, Deserialize)]
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

#[derive(Debug, Subcommand, PartialEq, Serialize, Deserialize)]
enum Command {
    /// Create new Parser state
    New {
        /// Program Name
        name: String,
        /// Program description
        #[arg(short, long)]
        description: Option<String>,
        /// Help text epilogue
        #[arg(short, long)]
        epilogue: Option<String>,
    },
    /// Add an argument
    #[command(name = "add_arg")]
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
    #[command(name = "subparser_init")]
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
    #[command(name = "set_defaults")]
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
            let cmd: Command = qs::from_str(act).unwrap();
            match cmd {
                Command::New {
                    name,
                    description,
                    epilogue,
                } => {
                    let parser_args = PyTuple::new_bound(py, vec![name]);

                    let parser_kwargs = PyDict::new_bound(py);
                    if let Some(v) = description {
                        parser_kwargs.set_item("description", v)?;
                    }
                    if let Some(v) = epilogue {
                        parser_kwargs.set_item("epilogue", v)?;
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
            let s = qs::to_string(&cli.command).unwrap();
            print!("{}{}", DELIMITER, s);
        }
    }
}

use std::collections::HashMap;

use crate::{
    Action as CliAction, AddArgCommand, AddSubcommandCommand, AddSubparserCommand,
    NArgs as CliNArgs,
};

#[derive(Debug, Clone, PartialEq)]
pub enum Nargs {
    Exact(usize),
    AtLeastOne,
    Many,
}

impl From<CliNArgs> for Nargs {
    fn from(v: CliNArgs) -> Self {
        match v {
            CliNArgs::AtLeastOne => Nargs::AtLeastOne,
            CliNArgs::Many => Nargs::Many,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Action {
    Store,
    StoreTrue,
    Append,
    Count,
    StoreConst,
    AppendConst,
    Version,
    Help,
}

impl From<CliAction> for Action {
    fn from(v: CliAction) -> Self {
        match v {
            CliAction::Store => Action::Store,
            CliAction::StoreTrue => Action::StoreTrue,
            CliAction::Append => Action::Append,
            CliAction::Count => Action::Count,
            CliAction::Help => Action::Help,
            _ => Action::Store,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ArgValue {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Null,
    List(Vec<ArgValue>),
}

impl From<ArgValue> for serde_json::Value {
    fn from(v: ArgValue) -> Self {
        match v {
            ArgValue::String(s) => serde_json::Value::String(s),
            ArgValue::Int(i) => serde_json::Value::Number(i.into()),
            ArgValue::Float(f) => serde_json::Value::Number(
                serde_json::Number::from_f64(f).unwrap_or(serde_json::Number::from(0)),
            ),
            ArgValue::Bool(b) => serde_json::Value::Bool(b),
            ArgValue::Null => serde_json::Value::Null,
            ArgValue::List(lst) => {
                serde_json::Value::Array(lst.iter().map(|x| x.clone().into()).collect())
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum ArgType {
    Int,
    Float,
}

#[derive(Debug, Clone)]
pub struct Argument {
    pub name: String,
    pub dest: String,
    pub options: Vec<String>,
    pub positional: bool,
    pub nargs: Nargs,
    pub action: Action,
    pub default: Option<ArgValue>,
    pub type_: Option<ArgType>,
    pub choices: Option<Vec<ArgValue>>,
    pub required: bool,
    pub help: Option<String>,
    pub metavar: Option<String>,
    pub deprecated: bool,
    pub const_: Option<ArgValue>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PosMarker {
    Arg(usize),
    Subparser(usize),
}

#[derive(Debug, Clone, Copy)]
pub enum PathStep {
    Sub(usize),
    Cmd(usize),
}

#[derive(Debug, Clone)]
pub struct Subparser {
    pub dest: String,
    pub required: bool,
    pub help: Option<String>,
    pub metavar: Option<String>,
    pub commands: Vec<Subcommand>,
}

#[derive(Debug, Clone)]
pub struct Subcommand {
    pub name: String,
    pub help: Option<String>,
    pub parser: Parser,
}

#[derive(Debug, Clone)]
pub struct Parser {
    pub name: String,
    pub description: Option<String>,
    pub epilog: Option<String>,
    pub arguments: Vec<Argument>,
    pub subparsers: Vec<Subparser>,
    pub positional_order: Vec<PosMarker>,
    pub defaults: HashMap<String, ArgValue>,
    pub current_subparser_path: Vec<PathStep>,
}

#[derive(Debug)]
pub enum ParseResult {
    Success(HashMap<String, ArgValue>),
    Help(String),
    Error(String),
    Version(String),
}

impl Parser {
    pub fn new() -> Self {
        Parser {
            name: String::new(),
            description: None,
            epilog: None,
            arguments: Vec::new(),
            subparsers: Vec::new(),
            positional_order: Vec::new(),
            defaults: HashMap::new(),
            current_subparser_path: Vec::new(),
        }
    }

    pub fn initialize(
        &mut self,
        name: String,
        description: Option<String>,
        epilog: Option<String>,
    ) {
        self.name = name;
        self.description = description;
        self.epilog = epilog;
        // argparse automatically adds the -h/--help action
        self.arguments.push(Argument {
            name: "--help".to_string(),
            dest: "help".to_string(),
            options: vec!["-h".to_string(), "--help".to_string()],
            positional: false,
            nargs: Nargs::Exact(0),
            action: Action::Help,
            default: None,
            type_: None,
            choices: None,
            required: false,
            help: Some("show this help message and exit".to_string()),
            metavar: None,
            deprecated: false,
            const_: None,
        });
    }

    fn subparser_idx_by_id(&self, id: &str) -> usize {
        self.subparsers
            .iter()
            .position(|s| s.dest == id)
            .expect("subparser not found")
    }

    fn subparser_at(&self, path: &[PathStep]) -> &Subparser {
        let mut node = Node::P(self);
        for step in path {
            node = match (node, step) {
                (Node::P(p), PathStep::Sub(i)) => Node::S(&p.subparsers[*i]),
                (Node::S(s), PathStep::Cmd(i)) => Node::P(&s.commands[*i].parser),
                _ => panic!("bad subparser path"),
            };
        }
        match node {
            Node::S(s) => s,
            _ => panic!("expected subparser at end of path"),
        }
    }

    fn parser_at(&self, path: &[PathStep]) -> &Parser {
        let mut node = Node::P(self);
        for step in path {
            node = match (node, step) {
                (Node::P(p), PathStep::Sub(i)) => Node::S(&p.subparsers[*i]),
                (Node::S(s), PathStep::Cmd(i)) => Node::P(&s.commands[*i].parser),
                _ => panic!("bad parser path"),
            };
        }
        match node {
            Node::P(p) => p,
            _ => panic!("expected parser at end of path"),
        }
    }

    fn subparser_at_mut(&mut self, path: &[PathStep]) -> &mut Subparser {
        let mut node = NodeMut::P(self);
        for step in path {
            node = match (node, step) {
                (NodeMut::P(p), PathStep::Sub(i)) => NodeMut::S(&mut p.subparsers[*i]),
                (NodeMut::S(s), PathStep::Cmd(i)) => NodeMut::P(&mut s.commands[*i].parser),
                _ => panic!("bad subparser path"),
            };
        }
        match node {
            NodeMut::S(s) => s,
            _ => panic!("expected subparser at end of path"),
        }
    }

    fn parser_at_mut(&mut self, path: &[PathStep]) -> &mut Parser {
        let mut node = NodeMut::P(self);
        for step in path {
            node = match (node, step) {
                (NodeMut::P(p), PathStep::Sub(i)) => NodeMut::S(&mut p.subparsers[*i]),
                (NodeMut::S(s), PathStep::Cmd(i)) => NodeMut::P(&mut s.commands[*i].parser),
                _ => panic!("bad parser path"),
            };
        }
        match node {
            NodeMut::P(p) => p,
            _ => panic!("expected parser at end of path"),
        }
    }

    /// Path (from the root) to the parser an argument/defaults should be attached to.
    /// Returns [] (the main parser) when no subcommand is targeted.
    fn arg_target_path(
        &self,
        subcommand: &Option<String>,
        subparserid: &Option<String>,
    ) -> Vec<PathStep> {
        match subcommand {
            None => Vec::new(),
            Some(cmd) => {
                let sp_path = if let Some(spid) = subparserid {
                    vec![PathStep::Sub(self.subparser_idx_by_id(spid))]
                } else {
                    self.current_subparser_path.clone()
                };
                let sp = self.subparser_at(&sp_path);
                let cmd_idx = sp
                    .commands
                    .iter()
                    .position(|c| c.name == *cmd)
                    .expect("subcommand not found");
                let mut path = sp_path;
                path.push(PathStep::Cmd(cmd_idx));
                path
            }
        }
    }
}

#[derive(Debug)]
enum Node<'a> {
    P(&'a Parser),
    S(&'a Subparser),
}

#[derive(Debug)]
enum NodeMut<'a> {
    P(&'a mut Parser),
    S(&'a mut Subparser),
}

impl Parser {
    pub fn add_argument(&mut self, opts: AddArgCommand) {
        let args = opts.args.clone().unwrap_or_default();
        let options: Vec<String> = args
            .iter()
            .filter(|a| a.starts_with('-'))
            .cloned()
            .collect();
        let positional = options.is_empty();
        let name = if args.len() == 1 {
            args.first().cloned().unwrap_or_default()
        } else {
            args.iter()
                .find(|a| a.starts_with("--"))
                .cloned()
                .unwrap_or_else(|| args.first().cloned().unwrap_or_default())
        };
        let name = name.trim_start_matches('-').to_owned();

        let dest = opts.dest.clone().unwrap_or_else(|| name.clone());

        let mut nargs = match opts.nargs_exact {
            Some(n) => Nargs::Exact(n),
            None => opts.nargs.map(Nargs::from).unwrap_or(Nargs::Exact(1)),
        };

        let action = match opts.action {
            Some(a) => a.into(),
            None => {
                if opts.store_const.is_some() {
                    Action::StoreConst
                } else if opts.append_const.is_some() {
                    Action::AppendConst
                } else if opts.displays_version {
                    Action::Version
                } else {
                    Action::Store
                }
            }
        };

        match action {
            Action::AppendConst
            | Action::Count
            | Action::Help
            | Action::StoreConst
            | Action::StoreTrue
            | Action::Version => {
                nargs = Nargs::Exact(0);
            }
            _ => {}
        };

        let type_ = opts.type_.map(|t| parse_type(&t));
        let default = opts.default.map(|d| parse_value(&d, &type_));
        let choices = opts
            .choice
            .map(|c| c.into_iter().map(|c| parse_value(&c, &type_)).collect());

        let arg = Argument {
            name,
            dest,
            options,
            positional,
            nargs,
            action,
            default,
            type_: type_.clone(),
            choices,
            required: opts.required,
            help: opts.helptext,
            metavar: opts.metavar,
            deprecated: opts.deprecated,
            const_: opts.store_const.map(|c| parse_value(&c, &type_)),
        };

        let target = self.parser_at_mut(&self.arg_target_path(&opts.subcommand, &opts.subparserid));
        let idx = target.arguments.len();
        target.arguments.push(arg);
        if positional {
            target.positional_order.push(PosMarker::Arg(idx));
        }
    }

    pub fn add_subparser(&mut self, opts: AddSubparserCommand) {
        let name = opts.name.clone();
        let dest = opts.dest.clone().unwrap_or_else(|| name.clone());
        let is_main = opts.subcommand.is_none();

        let parent_path: Vec<PathStep> = if is_main {
            Vec::new()
        } else {
            self.arg_target_path(&opts.subcommand, &opts.subparserid)
        };

        let new_sp = Subparser {
            dest,
            required: opts.required,
            help: opts.helptext,
            metavar: opts.metavar,
            commands: Vec::new(),
        };

        let idx;
        {
            let parent = self.parser_at_mut(&parent_path);
            idx = parent.subparsers.len();
            parent.positional_order.push(PosMarker::Subparser(idx));
            parent.subparsers.push(new_sp);
        }

        self.current_subparser_path = if is_main {
            vec![PathStep::Sub(idx)]
        } else {
            {
                let mut p = parent_path;
                p.push(PathStep::Sub(idx));
                p
            }
        };
    }

    pub fn add_subcommand(&mut self, opts: AddSubcommandCommand) {
        let name = opts.name.clone();
        let helptext = opts.helptext.clone();

        let sp_path: Vec<PathStep> = if let Some(spid) = opts.subparserid {
            vec![PathStep::Sub(self.subparser_idx_by_id(&spid))]
        } else {
            let cur = self.current_subparser_path.clone();
            if cur.is_empty() {
                panic!("add_subcommand: no current subparser (run add_subparser first)")
            }
            cur
        };

        let mut sc = Subcommand {
            name: name.clone(),
            help: helptext,
            parser: Parser::new(),
        };
        sc.parser.name = name.clone();
        let mut sp = self.subparser_at_mut(&sp_path);
        sp.commands.push(sc);
    }

    pub fn set_defaults(
        &mut self,
        subcommand: Option<String>,
        subparserid: Option<String>,
        args: Option<Vec<String>>,
    ) {
        let args = args.unwrap_or_default();
        let pairs: Vec<(String, String)> = args
            .chunks(2)
            .filter(|c| c.len() == 2)
            .map(|c| {
                // Mirror Python's arglist_to_kwargs: keys are `--`-prefixed and
                // the prefix is stripped (`key = arglist[i][2:]`).
                let key = if c[0].starts_with("--") {
                    c[0][2..].to_string()
                } else {
                    c[0].clone()
                };
                (key, c[1].clone())
            })
            .collect();

        let path = self.arg_target_path(&subcommand, &subparserid);
        let mut target = self.parser_at_mut(&path);
        for (k, v) in pairs {
            target.defaults.insert(k.clone(), parse_value(&v, &None));
        }
    }
}

impl Parser {
    pub fn parse_args(&self, args: Vec<String>) -> ParseResult {
        match self.parse_prefix(&args) {
            InternalResult::Success { kv, consumed } => {
                if consumed == args.len() {
                    let map: std::collections::HashMap<String, ArgValue> = kv.into_iter().collect();
                    ParseResult::Success(map)
                } else {
                    let remaining: Vec<&str> =
                        args[consumed..].iter().map(|s| s.as_str()).collect();
                    ParseResult::Error(format!(
                        "{}: error: unrecognized arguments: {}",
                        self.name,
                        remaining.join(" ")
                    ))
                }
            }
            InternalResult::Help(h) => ParseResult::Help(h),
            InternalResult::Error(e) => ParseResult::Error(e),
            InternalResult::Version(v) => ParseResult::Version(v),
        }
    }

    fn parse_prefix(&self, args: &[String]) -> InternalResult {
        let mut opt_alias: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        for (i, a) in self.arguments.iter().enumerate() {
            if !a.positional {
                for opt in &a.options {
                    opt_alias.entry(opt.clone()).or_insert(i);
                }
            }
        }

        let mut kv: Vec<(String, ArgValue)> = self.namespace();
        let mut pos_cursor: usize = 0;
        let mut i: usize = 0;
        while i < args.len() {
            let token = &args[i];
            let is_opt = token.starts_with('-');
            if is_opt {
                if token == "-h" || token == "--help" {
                    return InternalResult::Help(self.format_help());
                }
                let consumed = match opt_alias.get(token) {
                    Some(&j) => {
                        let a = &self.arguments[j];
                        match a.action {
                            Action::Help => {
                                return InternalResult::Help(self.format_help());
                            }
                            Action::Version => {
                                return InternalResult::Version(self.version_value(a));
                            }
                            Action::Store => {
                                if i + 1 >= args.len() {
                                    return InternalResult::Error(format!(
                                        "{}: error: option {}: requires an argument",
                                        self.name, token
                                    ));
                                }
                                let val_token = &args[i + 1];
                                match self.convert_value(val_token, a) {
                                    Some(val) => {
                                        push_kv(&mut kv, a.dest.clone(), val);
                                        2
                                    }
                                    None => {
                                        return InternalResult::Error(
                                            self.invalid_value_message(a, val_token),
                                        );
                                    }
                                }
                            }
                            Action::StoreTrue => {
                                push_kv(&mut kv, a.dest.clone(), ArgValue::Bool(true));
                                1
                            }
                            Action::Count => {
                                match kv.iter().position(|(k, _)| k == &a.dest) {
                                    Some(pos) => match &mut kv[pos].1 {
                                        ArgValue::Int(c) => *c += 1,
                                        _ => kv[pos].1 = ArgValue::Int(1),
                                    },
                                    None => push_kv(&mut kv, a.dest.clone(), ArgValue::Int(1)),
                                }
                                1
                            }
                            Action::StoreConst => {
                                let val = a.const_.clone().unwrap_or(ArgValue::Null);
                                push_kv(&mut kv, a.dest.clone(), val);
                                1
                            }
                            Action::AppendConst => {
                                let val = a.const_.clone().unwrap_or(ArgValue::Null);
                                append_kv(&mut kv, a.dest.clone(), val);
                                1
                            }
                            Action::Append => {
                                if i + 1 >= args.len() {
                                    return InternalResult::Error(format!(
                                        "{}: error: option {}: requires an argument",
                                        self.name, token
                                    ));
                                }
                                let val_token = &args[i + 1];
                                match self.convert_value(val_token, a) {
                                    Some(val) => {
                                        append_kv(&mut kv, a.dest.clone(), val);
                                        2
                                    }
                                    None => {
                                        return InternalResult::Error(
                                            self.invalid_value_message(a, val_token),
                                        );
                                    }
                                }
                            }
                            _ => 1,
                        }
                    }
                    None => {
                        return InternalResult::Error(format!(
                            "{}: error: option {}: is not recognized",
                            self.name, token
                        ));
                    }
                };
                i += consumed;
                continue;
            }
            // positional
            if pos_cursor >= self.positional_order.len() {
                break;
            }
            match self.positional_order[pos_cursor] {
                PosMarker::Arg(idx) => {
                    let a = &self.arguments[idx];
                    match a.nargs {
                        Nargs::Exact(1) => {
                            if i + 1 > args.len() {
                                break;
                            }
                            match self.convert_value(&args[i], a) {
                                Some(v) => {
                                    push_kv(&mut kv, a.dest.clone(), v);
                                    pos_cursor += 1;
                                    i += 1;
                                }
                                None => {
                                    return InternalResult::Error(
                                        self.invalid_value_message(a, &args[i]),
                                    )
                                }
                            }
                        }
                        Nargs::Exact(n) => {
                            if i + n > args.len() {
                                break;
                            }
                            let vals: Vec<Option<ArgValue>> = args[i..i + n]
                                .iter()
                                .map(|t| self.convert_value(t, a))
                                .collect();
                            if vals.iter().all(|v| v.is_some()) {
                                let vals: Vec<ArgValue> =
                                    vals.into_iter().map(|v| v.unwrap()).collect();
                                let final_val = if vals.len() == 1 {
                                    vals[0].clone()
                                } else {
                                    ArgValue::List(vals)
                                };
                                push_kv(&mut kv, a.dest.clone(), final_val);
                                pos_cursor += 1;
                                i += n;
                            } else {
                                return InternalResult::Error(
                                    self.invalid_value_message(a, &args[i]),
                                );
                            }
                        }
                        Nargs::AtLeastOne => {
                            if i + 1 > args.len() {
                                break;
                            }
                            match self.convert_value(&args[i], a) {
                                Some(v) => {
                                    push_kv(&mut kv, a.dest.clone(), v);
                                    pos_cursor += 1;
                                    i += 1;
                                }
                                None => {
                                    return InternalResult::Error(
                                        self.invalid_value_message(a, &args[i]),
                                    )
                                }
                            }
                        }
                        Nargs::Many => {
                            let n = args.len() - i;
                            if n == 0 {
                                break;
                            }
                            let vals: Vec<Option<ArgValue>> = args[i..i + n]
                                .iter()
                                .map(|t| self.convert_value(t, a))
                                .collect();
                            if vals.iter().all(|v| v.is_some()) {
                                let vals: Vec<ArgValue> =
                                    vals.into_iter().map(|v| v.unwrap()).collect();
                                let final_val = if vals.len() == 1 {
                                    vals[0].clone()
                                } else {
                                    ArgValue::List(vals)
                                };
                                push_kv(&mut kv, a.dest.clone(), final_val);
                                pos_cursor += 1;
                                i += n;
                            } else {
                                return InternalResult::Error(
                                    self.invalid_value_message(a, &args[i]),
                                );
                            }
                        }
                    }
                }
                PosMarker::Subparser(si) => {
                    let sp = &self.subparsers[si];
                    let ci = sp.commands.iter().position(|c| c.name == args[i].as_str());
                    if let Some(ci) = ci {
                        let sub = &sp.commands[ci].parser;
                        let remaining = &args[i + 1..];
                        let sub_res = sub.parse_prefix(remaining);
                        match sub_res {
                            InternalResult::Success {
                                kv: sub_kv,
                                consumed: sub_consumed,
                            } => {
                                // Set the subcommand name first, matching Python
                                // argparse where `setattr(namespace, dest, parser_name)`
                                // happens before the subparser's own defaults are merged
                                // in. This lets `set_defaults --command ...` on a
                                // subcommand override the subcommand name.
                                push_kv(
                                    &mut kv,
                                    sp.dest.clone(),
                                    ArgValue::String(sp.commands[ci].name.clone()),
                                );
                                for (k, v) in sub_kv {
                                    push_kv(&mut kv, k, v);
                                }
                                i += 1 + sub_consumed;
                                pos_cursor += 1;
                            }
                            InternalResult::Help(h) => return InternalResult::Help(h),
                            InternalResult::Error(e) => return InternalResult::Error(e),
                            InternalResult::Version(v) => return InternalResult::Version(v),
                        }
                    } else {
                        if sp.required {
                            return InternalResult::Error(format!(
                                "{}: error: invalid subcommand '{}'. Available commands: {}",
                                self.name,
                                args[i].as_str(),
                                sp.commands
                                    .iter()
                                    .map(|s| s.name.clone())
                                    .collect::<Vec<String>>()
                                    .join(", ")
                            ));
                        } else {
                            pos_cursor += 1;
                            i += 1;
                        }
                    }
                }
            }
        }
        let missing = self.missing_required(&kv, pos_cursor);
        if !missing.is_empty() {
            return InternalResult::Error(format!(
                "{}: error: the following arguments are required: {}",
                self.name,
                missing.join(" ")
            ));
        }
        InternalResult::Success { kv, consumed: i }
    }

    fn namespace(&self) -> Vec<(String, ArgValue)> {
        let mut kv = Vec::new();
        for a in &self.arguments {
            if let Some(d) = &a.default {
                push_kv(&mut kv, a.dest.clone(), d.clone());
            }
        }
        for sp in &self.subparsers {
            push_kv(&mut kv, sp.dest.clone(), ArgValue::Null);
        }
        for (k, v) in &self.defaults {
            push_kv(&mut kv, k.clone(), v.clone());
        }
        kv
    }

    fn missing_required(&self, kv: &[(String, ArgValue)], pos_cursor: usize) -> Vec<String> {
        let in_kv = |k: &str| kv.iter().any(|(k2, _)| k2 == k);
        let mut missing = Vec::new();
        for a in &self.arguments {
            if !a.positional && a.required && !in_kv(&a.dest) {
                missing.push(a.dest.clone());
            }
        }
        for (k, marker) in self.positional_order.iter().enumerate() {
            if k >= pos_cursor {
                match marker {
                    PosMarker::Arg(idx) => {
                        if !in_kv(&self.arguments[*idx].dest) {
                            missing.push(self.arguments[*idx].dest.clone());
                        }
                    }
                    PosMarker::Subparser(si) => {
                        let sp = &self.subparsers[*si];
                        // `namespace()` pre-populates the subparser dest with `Null`,
                        // so "not dispatched" is detected by a `Null` value rather than
                        // absence. Only required subparsers raise a missing-arg error
                        // when omitted (non-required ones may be left unset).
                        let dispatched = kv
                            .iter()
                            .find(|(k2, _)| k2 == &sp.dest)
                            .map(|(_, v)| !matches!(v, ArgValue::Null))
                            .unwrap_or(false);
                        if sp.required && !dispatched {
                            missing.push(sp.dest.clone());
                        }
                    }
                }
            }
        }
        missing
    }

    fn version_value(&self, a: &Argument) -> String {
        match a.const_.as_ref() {
            Some(ArgValue::String(s)) => s.clone(),
            Some(ArgValue::Int(i)) => i.to_string(),
            Some(ArgValue::Float(f)) => f.to_string(),
            Some(ArgValue::Bool(b)) => b.to_string(),
            _ => String::new(),
        }
    }

    fn convert_value(&self, token: &str, a: &Argument) -> Option<ArgValue> {
        if let Some(choices) = &a.choices {
            for c in choices {
                if choice_matches(c, token) {
                    return Some(c.clone());
                }
            }
            return None;
        }
        if let Some(t) = &a.type_ {
            match t {
                ArgType::Int => token.parse::<i64>().ok().map(ArgValue::Int),
                ArgType::Float => token.parse::<f64>().ok().map(ArgValue::Float),
            }
        } else {
            Some(ArgValue::String(token.to_string()))
        }
    }

    fn invalid_value_message(&self, a: &Argument, token: &str) -> String {
        if a.choices.is_some() {
            let from = a
                .choices
                .as_ref()
                .unwrap()
                .iter()
                .map(choice_display)
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "{}: error: argument {}: invalid choice: '{}' (choose from '{}')",
                self.name, a.name, token, from
            )
        } else if let Some(t) = &a.type_ {
            match t {
                ArgType::Int => format!(
                    "{}: error: argument {}: invalid int value: '{}'",
                    self.name, a.name, token
                ),
                ArgType::Float => format!(
                    "{}: error: argument {}: invalid float value: '{}'",
                    self.name, a.name, token
                ),
            }
        } else {
            format!(
                "{}: error: argument {}: invalid value: '{}'",
                self.name, a.name, token
            )
        }
    }
}

#[derive(Debug)]
enum InternalResult {
    Success {
        kv: Vec<(String, ArgValue)>,
        consumed: usize,
    },
    Help(String),
    Error(String),
    Version(String),
}

fn push_kv(kv: &mut Vec<(String, ArgValue)>, key: String, val: ArgValue) {
    if let Some(pos) = kv.iter().position(|(k, _)| *k == key) {
        kv[pos].1 = val;
    } else {
        kv.push((key, val));
    }
}

fn append_kv(kv: &mut Vec<(String, ArgValue)>, key: String, val: ArgValue) {
    if let Some(pos) = kv.iter().position(|(k, _)| *k == key) {
        let slot = &mut kv[pos].1;
        match slot {
            ArgValue::List(lst) => {
                lst.push(val);
            }
            _ => {
                let old = slot.clone();
                *slot = ArgValue::List(vec![old, val]);
            }
        }
    } else {
        kv.push((key, ArgValue::List(vec![val])));
    }
}

fn choice_matches(c: &ArgValue, token: &str) -> bool {
    match c {
        ArgValue::String(s) => s == token,
        ArgValue::Int(i) => token == i.to_string(),
        ArgValue::Float(f) => token
            .parse::<f64>()
            .ok()
            .map(|v| (v - f).abs() < 1e-9)
            .unwrap_or(false),
        ArgValue::Bool(b) => {
            if *b {
                token == "True"
            } else {
                token == "False"
            }
        }
        ArgValue::Null => token == "null",
        ArgValue::List(_) => false,
    }
}

fn choice_display(c: &ArgValue) -> String {
    match c {
        ArgValue::String(s) => s.clone(),
        ArgValue::Int(i) => i.to_string(),
        ArgValue::Float(f) => f.to_string(),
        ArgValue::Bool(b) => b.to_string(),
        ArgValue::Null => "null".to_string(),
        ArgValue::List(_) => "list".to_string(),
    }
}

fn parse_type(t: &str) -> ArgType {
    match t {
        "int" => ArgType::Int,
        "float" => ArgType::Float,
        _ => ArgType::Int,
    }
}

fn parse_value(v: &str, t: &Option<ArgType>) -> ArgValue {
    match t {
        Some(ArgType::Int) => v
            .parse::<i64>()
            .ok()
            .map_or_else(|| ArgValue::String(v.to_string()), ArgValue::Int),
        Some(ArgType::Float) => v
            .parse::<f64>()
            .ok()
            .map_or_else(|| ArgValue::String(v.to_string()), ArgValue::Float),
        _ => ArgValue::String(v.to_string()),
    }
}

impl Parser {
    /// Build the `usage: ...` line (without a trailing newline).
    pub fn usage_line(&self) -> String {
        let optional_args: Vec<&Argument> =
            self.arguments.iter().filter(|a| !a.positional).collect();
        let positional_order: Vec<&Argument> = self
            .positional_order
            .iter()
            .filter_map(|m| match m {
                PosMarker::Arg(i) => {
                    let a = &self.arguments[*i];
                    if a.positional {
                        Some(a)
                    } else {
                        None
                    }
                }
                _ => None,
            })
            .collect();

        let mut usage = String::from("usage: ");
        if !self.name.is_empty() {
            usage.push_str(&self.name);
        }
        for a in &optional_args {
            usage.push(' ');
            usage.push_str(&format_usage_arg(a));
        }
        for a in &positional_order {
            usage.push(' ');
            usage.push_str(&format_usage_arg(a));
        }
        for sp in &self.subparsers {
            usage.push(' ');
            usage.push_str(&format_subparser_usage(sp));
        }
        usage
    }

    pub fn format_help(&self) -> String {
        // Compute the action max length across all actions.
        let action_max_length = self
            .arguments
            .iter()
            .map(|a| format_action_invocation(a).len() + 2)
            .max()
            .unwrap_or(0);
        let help_position = std::cmp::min(action_max_length + 2, 24);

        let optional_args: Vec<&Argument> =
            self.arguments.iter().filter(|a| !a.positional).collect();
        let positional_order: Vec<Argument> = self
            .positional_order
            .iter()
            .filter(|m| match m {
                PosMarker::Arg(i) => self.arguments[*i].positional,
                _ => false,
            })
            .map(|m| match m {
                PosMarker::Arg(i) => self.arguments[*i].clone(),
                _ => self.arguments[0].clone(),
            })
            .collect();

        // Build the usage line.
        let usage = self.usage_line();

        // Assemble the help text as a list of sections (each a list of
        // lines), then join sections with a blank line and terminate with a
        // single newline - mirroring Python argparse's HelpFormatter.
        let line = |t: String| t.trim_end().to_string();
        let mut sections: Vec<Vec<String>> = Vec::new();
        sections.push(vec![usage]);

        if let Some(desc) = &self.description {
            if !desc.is_empty() {
                sections.push(desc.lines().map(|l| l.to_string()).collect());
            }
        }

        if !positional_order.is_empty() {
            let mut section = vec!["positional arguments:".to_string()];
            for a in &positional_order {
                section.push(line(format_arg_line(a, help_position)));
            }
            sections.push(section);
        }

        if !self.subparsers.is_empty() {
            let mut section = vec!["subparsers:".to_string()];
            for sp in &self.subparsers {
                section.push(line(format_subparser_line(sp, help_position)));
            }
            sections.push(section);
        }

        if !optional_args.is_empty() {
            let mut section = vec!["options:".to_string()];
            for a in &optional_args {
                section.push(line(format_arg_line(a, help_position)));
            }
            sections.push(section);
        }

        if let Some(epilog) = &self.epilog {
            if !epilog.is_empty() {
                sections.push(epilog.lines().map(|l| l.to_string()).collect());
            }
        }

        let mut out = String::new();
        for (i, section) in sections.iter().enumerate() {
            if i > 0 {
                out.push('\n'); // blank line separating sections
            }
            out.push_str(&section.join("\n"));
            out.push('\n');
        }
        out
    }
}

fn choices_metavar(choices: &[ArgValue]) -> String {
    let joined = choices
        .iter()
        .map(choice_display)
        .collect::<Vec<_>>()
        .join(",");
    format!("{{{}}}", joined)
}

fn base_metavar(a: &Argument) -> String {
    if let Some(m) = &a.metavar {
        return m.clone();
    }
    if let Some(choices) = &a.choices {
        return choices_metavar(choices);
    }
    if a.positional {
        a.name.clone()
    } else {
        a.dest.to_uppercase()
    }
}

fn get_metavar(a: &Argument) -> String {
    if let Some(m) = &a.metavar {
        return m.clone();
    }
    if let Some(choices) = &a.choices {
        return choices_metavar(choices);
    }
    let base = base_metavar(a);
    match a.nargs {
        Nargs::Exact(0) => base,
        Nargs::Exact(1) => base,
        Nargs::Exact(n) => (0..n).map(|_| base.clone()).collect::<Vec<_>>().join(" "),
        Nargs::AtLeastOne => format!("{} [{} ...]", base, base),
        Nargs::Many => format!("[{} ...]", base),
    }
}

/// Python argparse shows every option string in the options list, joined by
/// ", " (e.g. "-h, --help").
fn format_action_invocation(a: &Argument) -> String {
    if a.positional {
        match a.nargs {
            Nargs::Exact(0) => String::new(),
            _ => base_metavar(a),
        }
    } else {
        let options = a.options.join(", ");
        if a.nargs == Nargs::Exact(0) {
            options
        } else {
            format!("{} {}", options, get_metavar(a))
        }
    }
}

/// Python argparse shows only the short option (a single-dash flag) in the
/// usage line, falling back to the first option string when no short form
/// exists (e.g. "-h" for "-h, --help", or "--intarg" for a long-only option).
fn usage_option(a: &Argument) -> String {
    a.options
        .iter()
        .find(|o| o.len() == 2 && o.starts_with('-'))
        .cloned()
        .unwrap_or_else(|| a.options.first().cloned().unwrap_or_default())
}

fn format_usage_arg(a: &Argument) -> String {
    if a.positional {
        match a.nargs {
            Nargs::Exact(0) => String::new(),
            _ => get_metavar(a),
        }
    } else {
        let option = usage_option(a);
        if a.nargs == Nargs::Exact(0) {
            format!("[{}]", option)
        } else {
            format!("[{} {}]", option, get_metavar(a))
        }
    }
}

fn subparser_metavar(sp: &Subparser) -> String {
    sp.metavar.clone().unwrap_or_else(|| {
        if sp.commands.is_empty() {
            sp.dest.clone()
        } else {
            let cmds = sp
                .commands
                .iter()
                .map(|c| c.name.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            let mut s = String::new();
            s.push('{');
            s.push_str(&cmds);
            s.push('}');
            s
        }
    })
}

fn format_subparser_usage(sp: &Subparser) -> String {
    let mv = subparser_metavar(sp);
    if sp.required {
        mv
    } else {
        format!("[{}]", mv)
    }
}

fn format_subparser_line(sp: &Subparser, help_position: usize) -> String {
    let metavar = subparser_metavar(sp);
    let action_width = help_position - 4;
    let indent = "  ";
    let (action_header_str, indent_first) = if metavar.len() <= action_width {
        (
            format!("{}{:width$}  ", indent, metavar, width = action_width),
            0,
        )
    } else {
        (format!("{}{}\n", indent, metavar), help_position)
    };
    let help_text = sp.help.clone().unwrap_or_default();
    if !help_text.is_empty() {
        let help_width = std::cmp::max(78 - help_position, 11);
        let wrapped = wrap_text(&help_text, help_width);
        let help_lines: Vec<&str> = wrapped.split('\n').collect();
        let mut help_str = String::new();
        for (i, line) in help_lines.iter().enumerate() {
            if i == 0 {
                help_str.push_str(&format!("{:w$}{}", line, w = indent_first));
            } else {
                help_str.push_str(&format!("{:w$}{}", line, w = help_position));
            }
        }
        format!("{}{}\n", action_header_str, help_str)
    } else {
        format!("{}\n", action_header_str)
    }
}

fn wrap_text(text: &str, width: usize) -> String {
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut lines: Vec<String> = Vec::new();
    let mut current_line = String::new();
    for word in &words {
        if current_line.is_empty() {
            current_line.push_str(word);
        } else if current_line.len() + 1 + word.len() <= width {
            current_line.push(' ');
            current_line.push_str(word);
        } else {
            lines.push(current_line.clone());
            current_line = word.to_string();
        }
    }
    if !current_line.is_empty() {
        lines.push(current_line);
    }
    lines.join("\n")
}

fn format_arg_line(a: &Argument, help_position: usize) -> String {
    let action_header = format_action_invocation(a);
    let action_width = help_position - 4;
    let indent = "  ";
    let (action_header_str, indent_first) = if action_header.len() <= action_width {
        (
            format!("{}{:width$}  ", indent, action_header, width = action_width),
            0,
        )
    } else {
        (format!("{}{}\n", indent, action_header), help_position)
    };
    let help_text = a.help.clone().unwrap_or_default();
    if !help_text.is_empty() {
        let help_width = std::cmp::max(78 - help_position, 11);
        let wrapped = wrap_text(&help_text, help_width);
        let help_lines: Vec<&str> = wrapped.split('\n').collect();
        let mut help_str = String::new();
        for (i, line) in help_lines.iter().enumerate() {
            if i == 0 {
                help_str.push_str(line);
            } else {
                help_str.push_str(&format!("{:w$}{}", line, w = help_position));
            }
        }
        format!("{}{}\n", action_header_str, help_str)
    } else {
        format!("{}{}\n", action_header_str, "")
    }
}

//! A `#![no_std]` build of argparsh's parser-construction commands.
//!
//! This binary supports `new`, `add_arg`, `add_subparser`, `add_subcommand`
//! and `set_defaults`, and emits exactly the same `&<urlencoded bitcode>`
//! chunks as `argparsh`, so its output can be fed to `argparsh parse`.
//! It does not support `parse`, and never prints help text.
//!
//! Command-line handling mirrors the clap configuration in argparsh's
//! `src/main.rs`; the data types below must stay field-for-field identical to
//! the ones there, since bitcode's encoding depends on their exact shape.
#![no_std]
#![no_main]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use bitcode::Encode;
use core::alloc::{GlobalAlloc, Layout};
use core::ffi::{c_char, c_int, CStr};

const DELIMITER: u8 = b'&';

// ---------------------------------------------------------------------------
// Runtime glue: allocator, panic handler, I/O
// ---------------------------------------------------------------------------

// The libc crate only emits link directives when built as part of std, so
// link the C library (which also provides the process entry point) ourselves.
#[link(name = "c")]
extern "C" {}

struct Malloc;

/// Alignment guaranteed by malloc on the platforms we care about.
#[cfg(target_pointer_width = "64")]
const MIN_ALIGN: usize = 16;
#[cfg(not(target_pointer_width = "64"))]
const MIN_ALIGN: usize = 8;

unsafe impl GlobalAlloc for Malloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if layout.align() <= MIN_ALIGN && layout.align() <= layout.size() {
            libc::malloc(layout.size()) as *mut u8
        } else {
            let mut out = core::ptr::null_mut();
            let align = layout.align().max(core::mem::size_of::<usize>());
            if libc::posix_memalign(&mut out, align, layout.size()) != 0 {
                return core::ptr::null_mut();
            }
            out as *mut u8
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        libc::free(ptr as *mut libc::c_void);
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        if layout.align() <= MIN_ALIGN && layout.align() <= new_size {
            return libc::realloc(ptr as *mut libc::c_void, new_size) as *mut u8;
        }
        let new_layout = Layout::from_size_align_unchecked(new_size, layout.align());
        let new_ptr = self.alloc(new_layout);
        if !new_ptr.is_null() {
            core::ptr::copy_nonoverlapping(ptr, new_ptr, layout.size().min(new_size));
            self.dealloc(ptr, layout);
        }
        new_ptr
    }
}

#[global_allocator]
static ALLOCATOR: Malloc = Malloc;

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    write_all(2, b"argparsh-nostd-demo: internal error\n");
    unsafe { libc::abort() }
}

/// The precompiled `alloc` crate references this symbol from its unwind tables,
/// and unoptimized builds keep that reference alive. We build with
/// `panic = "abort"` so nothing ever unwinds and it is never called.
#[no_mangle]
extern "C" fn rust_eh_personality() {}

fn write_all(fd: c_int, mut buf: &[u8]) -> bool {
    while !buf.is_empty() {
        let n = unsafe { libc::write(fd, buf.as_ptr() as *const libc::c_void, buf.len()) };
        if n < 0 {
            if unsafe { *libc::__errno_location() } == libc::EINTR {
                continue;
            }
            return false;
        }
        buf = &buf[n as usize..];
    }
    true
}

/// Print `error: <parts...>` to stderr and exit with status 2 (as clap does).
fn die(parts: &[&str]) -> ! {
    let mut msg = String::from("error: ");
    for p in parts {
        msg.push_str(p);
    }
    msg.push('\n');
    write_all(2, msg.as_bytes());
    unsafe { libc::exit(2) }
}

// ---------------------------------------------------------------------------
// Encoded types (must match argparsh's src/main.rs exactly)
// ---------------------------------------------------------------------------

#[allow(dead_code, clippy::upper_case_acronyms)] // only needed so that `Command::Parse` has the same shape
#[derive(Encode)]
enum Format {
    Shell,
    AssocArray,
    JSON,
    Custom(String),
}

#[derive(Copy, Clone, Encode)]
enum NArgs {
    AtLeastOne,
    Many,
}

#[derive(Copy, Clone, Encode)]
enum Action {
    Store,
    StoreTrue,
    Append,
    Count,
    Help,
}

#[derive(Encode)]
struct AddArgCommand {
    subcommand: Option<String>,
    subparserid: Option<String>,
    nargs_exact: Option<usize>,
    nargs: Option<NArgs>,
    default: Option<String>,
    action: Option<Action>,
    store_const: Option<String>,
    append_const: Option<String>,
    displays_version: bool,
    version: Option<String>,
    type_: Option<String>,
    choice: Option<Vec<String>>,
    required: bool,
    helptext: Option<String>,
    metavar: Option<String>,
    dest: Option<String>,
    deprecated: bool,
    args: Option<Vec<String>>,
}

#[derive(Encode)]
struct AddSubparserCommand {
    subparserid: Option<String>,
    name: String,
    dest: Option<String>,
    required: bool,
    helptext: Option<String>,
    metavar: Option<String>,
    subcommand: Option<String>,
    parent_subparserid: Option<String>,
}

#[derive(Encode)]
struct AddSubcommandCommand {
    subparserid: Option<String>,
    name: String,
    helptext: Option<String>,
}

#[allow(dead_code)] // `Parse` is never constructed here
#[derive(Encode)]
enum Command {
    New {
        name: String,
        description: Option<String>,
        epilog: Option<String>,
    },
    AddArg(AddArgCommand),
    AddSubparser(AddSubparserCommand),
    AddSubcommand(AddSubcommandCommand),
    SetDefaults {
        subcommand: Option<String>,
        subparserid: Option<String>,
        args: Option<Vec<String>>,
    },
    Parse {
        parser: String,
        format: Format,
        prefix: Option<String>,
        export: bool,
        local: bool,
        name: Option<String>,
        custom_format: Option<String>,
        custom_error: Option<String>,
        args: Option<Vec<String>>,
    },
}

// ---------------------------------------------------------------------------
// Minimal clap-compatible argument parsing
// ---------------------------------------------------------------------------

#[derive(Copy, Clone, PartialEq)]
enum Kind {
    /// Boolean flag, takes no value
    Flag,
    /// Takes exactly one value, may appear once
    Opt,
    /// Takes one value per occurrence, may appear many times
    Append,
}

struct Spec {
    long: &'static str,
    /// 0 when the option has no short form
    short: u8,
    kind: Kind,
}

const fn spec(long: &'static str, short: u8, kind: Kind) -> Spec {
    Spec { long, short, kind }
}

#[derive(Copy, Clone, PartialEq)]
enum Positional {
    /// A single required value that may not start with '-'
    Name,
    /// `trailing_var_arg = true, allow_hyphen_values = true`
    Trailing,
}

struct SubSpec {
    options: &'static [Spec],
    positional: Positional,
    /// Pairs of option indices that may not both be supplied
    conflicts: &'static [(usize, usize)],
    /// (a, b): supplying option a requires option b
    requires: &'static [(usize, usize)],
}

enum Slot {
    Unset,
    Flag,
    /// An option that was seen; `None` while still waiting for its value
    One(Option<String>),
    Many(Vec<String>),
}

struct Matches {
    slots: Vec<Slot>,
    positionals: Vec<String>,
}

impl Matches {
    fn is_set(&self, i: usize) -> bool {
        !matches!(self.slots[i], Slot::Unset)
    }

    fn flag(&self, i: usize) -> bool {
        matches!(self.slots[i], Slot::Flag)
    }

    fn opt(&mut self, i: usize) -> Option<String> {
        match core::mem::replace(&mut self.slots[i], Slot::Unset) {
            Slot::One(v) => v,
            _ => None,
        }
    }

    fn many(&mut self, i: usize) -> Option<Vec<String>> {
        match core::mem::replace(&mut self.slots[i], Slot::Unset) {
            Slot::Many(v) => Some(v),
            _ => None,
        }
    }

    fn trailing(&mut self) -> Option<Vec<String>> {
        let v = core::mem::take(&mut self.positionals);
        if v.is_empty() {
            None
        } else {
            Some(v)
        }
    }
}

fn no_help() -> ! {
    die(&["help is not available in argparsh-nostd-demo; use `argparsh --help`"])
}

fn value_required(sub: &SubSpec, i: usize) -> ! {
    die(&["a value is required for '--", sub.options[i].long, "' but none was supplied"])
}

/// Record that option `i` was seen, rejecting repeats of non-append options.
/// Value-taking options get their value later via `put_value`.
fn start_option(sub: &SubSpec, m: &mut Matches, i: usize) {
    let spec = &sub.options[i];
    match (spec.kind, &m.slots[i]) {
        (Kind::Append, Slot::Many(_)) => {}
        (_, Slot::Unset) => {
            m.slots[i] = match spec.kind {
                Kind::Flag => Slot::Flag,
                Kind::Opt => Slot::One(None),
                Kind::Append => Slot::Many(Vec::new()),
            }
        }
        _ => die(&["the argument '--", spec.long, "' cannot be used multiple times"]),
    }
}

fn put_value(m: &mut Matches, i: usize, value: &str) {
    match &mut m.slots[i] {
        Slot::One(v) => *v = Some(String::from(value)),
        Slot::Many(v) => v.push(String::from(value)),
        _ => unreachable!(),
    }
}

fn parse_sub(sub: &SubSpec, args: &[&str]) -> Matches {
    let mut m = Matches {
        slots: sub.options.iter().map(|_| Slot::Unset).collect(),
        positionals: Vec::new(),
    };
    let find_long = |name: &str| sub.options.iter().position(|s| s.long == name);
    let find_short = |c: char| {
        sub.options
            .iter()
            .position(|s| s.short != 0 && s.short as char == c)
    };
    let is_known_short = |c: char| c == 'h' || find_short(c).is_some();

    // Option waiting for its value in the next argument
    let mut pending: Option<usize> = None;
    // Set after '--' or once a trailing_var_arg positional has started
    let mut trailing = false;

    for &arg in args {
        if trailing {
            if sub.positional == Positional::Name && !m.positionals.is_empty() {
                die(&["unexpected argument '", arg, "' found"]);
            }
            m.positionals.push(String::from(arg));
            continue;
        }

        // Before any positional has been seen, a Trailing positional accepts
        // unknown hyphenated arguments as values.
        let hyphen_ok = sub.positional == Positional::Trailing;

        if arg == "--" {
            if let Some(i) = pending {
                value_required(sub, i);
            }
            trailing = true;
            continue;
        } else if let Some(body) = arg.strip_prefix("--") {
            let (name, value) = match body.find('=') {
                Some(eq) => (&body[..eq], Some(&body[eq + 1..])),
                None => (body, None),
            };
            if let Some(i) = find_long(name) {
                if let Some(p) = pending {
                    value_required(sub, p);
                }
                start_option(sub, &mut m, i);
                match (sub.options[i].kind, value) {
                    (Kind::Flag, Some(v)) => die(&[
                        "unexpected value '",
                        v,
                        "' for '--",
                        name,
                        "' found; no more were expected",
                    ]),
                    (Kind::Flag, None) => {}
                    (_, Some(v)) => put_value(&mut m, i, v),
                    (_, None) => pending = Some(i),
                }
                continue;
            } else if name == "help" {
                no_help();
            } else if !hyphen_ok {
                die(&["unexpected argument '", arg, "' found"]);
            }
            // Otherwise: treat as a value below
        } else if arg.len() > 1 && arg.starts_with('-') {
            let cluster = &arg[1..];
            if !hyphen_ok || cluster.chars().all(is_known_short) {
                for (off, c) in cluster.char_indices() {
                    if let Some(p) = pending {
                        value_required(sub, p);
                    }
                    if c == 'h' {
                        no_help();
                    }
                    let Some(i) = find_short(c) else {
                        die(&["unexpected argument '", arg, "' found"]);
                    };
                    start_option(sub, &mut m, i);
                    if sub.options[i].kind == Kind::Flag {
                        continue;
                    }
                    let rest = &cluster[off + c.len_utf8()..];
                    if let Some(v) = rest.strip_prefix('=') {
                        put_value(&mut m, i, v);
                    } else if !rest.is_empty() {
                        put_value(&mut m, i, rest);
                    } else {
                        pending = Some(i);
                    }
                    break;
                }
                continue;
            }
            // Otherwise: treat as a value below
        }

        if let Some(i) = pending.take() {
            put_value(&mut m, i, arg);
            continue;
        }

        match sub.positional {
            Positional::Trailing => trailing = true,
            Positional::Name if !m.positionals.is_empty() => {
                die(&["unexpected argument '", arg, "' found"])
            }
            Positional::Name => {}
        }
        m.positionals.push(String::from(arg));
    }

    if let Some(i) = pending {
        value_required(sub, i);
    }
    if sub.positional == Positional::Name && m.positionals.is_empty() {
        die(&["the following required arguments were not provided: <NAME>"]);
    }
    for &(a, b) in sub.conflicts {
        if m.is_set(a) && m.is_set(b) {
            die(&[
                "the argument '--",
                sub.options[a].long,
                "' cannot be used with '--",
                sub.options[b].long,
                "'",
            ]);
        }
    }
    for &(a, b) in sub.requires {
        if m.is_set(a) && !m.is_set(b) {
            die(&[
                "the following required arguments were not provided: --",
                sub.options[b].long,
            ]);
        }
    }
    m
}

fn parse_usize(s: &str, long: &str) -> usize {
    match s.parse() {
        Ok(v) => v,
        Err(_) => die(&["invalid value '", s, "' for '--", long, "'"]),
    }
}

fn parse_nargs(s: &str) -> NArgs {
    match s {
        "+" => NArgs::AtLeastOne,
        "*" => NArgs::Many,
        _ => die(&["invalid value '", s, "' for '--nargs' [possible values: +, *]"]),
    }
}

fn parse_action(s: &str) -> Action {
    match s {
        "store" => Action::Store,
        "store_true" => Action::StoreTrue,
        "append" => Action::Append,
        "count" => Action::Count,
        "help" => Action::Help,
        _ => die(&[
            "invalid value '",
            s,
            "' for '--action' [possible values: store, store_true, append, count, help]",
        ]),
    }
}

// --- new -------------------------------------------------------------------

mod new {
    pub const DESCRIPTION: usize = 0;
    pub const EPILOG: usize = 1;
}

static NEW: SubSpec = SubSpec {
    options: &[spec("description", b'd', Kind::Opt), spec("epilog", b'e', Kind::Opt)],
    positional: Positional::Name,
    conflicts: &[],
    requires: &[],
};

fn cmd_new(args: &[&str]) -> Command {
    use new::*;
    let mut m = parse_sub(&NEW, args);
    Command::New {
        name: m.positionals.pop().unwrap(),
        description: m.opt(DESCRIPTION),
        epilog: m.opt(EPILOG),
    }
}

// --- add_arg ---------------------------------------------------------------

mod add_arg {
    pub const SUBCOMMAND: usize = 0;
    pub const SUBPARSERID: usize = 1;
    pub const NARGS_EXACT: usize = 2;
    pub const NARGS: usize = 3;
    pub const DEFAULT: usize = 4;
    pub const ACTION: usize = 5;
    pub const STORE_CONST: usize = 6;
    pub const APPEND_CONST: usize = 7;
    pub const DISPLAYS_VERSION: usize = 8;
    pub const VERSION: usize = 9;
    pub const TYPE: usize = 10;
    pub const CHOICE: usize = 11;
    pub const REQUIRED: usize = 12;
    pub const HELPTEXT: usize = 13;
    pub const METAVAR: usize = 14;
    pub const DEST: usize = 15;
    pub const DEPRECATED: usize = 16;
}

static ADD_ARG: SubSpec = {
    use add_arg::*;
    SubSpec {
        options: &[
            spec("subcommand", 0, Kind::Opt),
            spec("subparserid", 0, Kind::Opt),
            spec("nargs-exact", b'n', Kind::Opt),
            spec("nargs", 0, Kind::Opt),
            spec("default", b'd', Kind::Opt),
            spec("action", b'a', Kind::Opt),
            spec("store-const", 0, Kind::Opt),
            spec("append-const", 0, Kind::Opt),
            spec("displays-version", 0, Kind::Flag),
            spec("version", 0, Kind::Opt),
            spec("type", b't', Kind::Opt),
            spec("choice", b'c', Kind::Append),
            spec("required", b'r', Kind::Flag),
            spec("helptext", 0, Kind::Opt),
            spec("metavar", 0, Kind::Opt),
            spec("dest", 0, Kind::Opt),
            spec("deprecated", 0, Kind::Flag),
        ],
        positional: Positional::Trailing,
        conflicts: &[
            (NARGS, NARGS_EXACT),
            (STORE_CONST, ACTION),
            (APPEND_CONST, ACTION),
            (APPEND_CONST, STORE_CONST),
            (DISPLAYS_VERSION, ACTION),
            (DISPLAYS_VERSION, STORE_CONST),
            (DISPLAYS_VERSION, APPEND_CONST),
            (DISPLAYS_VERSION, NARGS),
            (DISPLAYS_VERSION, NARGS_EXACT),
            (REQUIRED, DEFAULT),
        ],
        requires: &[
            (SUBPARSERID, SUBCOMMAND),
            (DISPLAYS_VERSION, VERSION),
            (VERSION, DISPLAYS_VERSION),
        ],
    }
};

fn cmd_add_arg(args: &[&str]) -> Command {
    use add_arg::*;
    let mut m = parse_sub(&ADD_ARG, args);
    Command::AddArg(AddArgCommand {
        subcommand: m.opt(SUBCOMMAND),
        subparserid: m.opt(SUBPARSERID),
        nargs_exact: m.opt(NARGS_EXACT).map(|s| parse_usize(&s, "nargs-exact")),
        nargs: m.opt(NARGS).map(|s| parse_nargs(&s)),
        default: m.opt(DEFAULT),
        action: m.opt(ACTION).map(|s| parse_action(&s)),
        store_const: m.opt(STORE_CONST),
        append_const: m.opt(APPEND_CONST),
        displays_version: m.flag(DISPLAYS_VERSION),
        version: m.opt(VERSION),
        type_: m.opt(TYPE),
        choice: m.many(CHOICE),
        required: m.flag(REQUIRED),
        helptext: m.opt(HELPTEXT),
        metavar: m.opt(METAVAR),
        dest: m.opt(DEST),
        deprecated: m.flag(DEPRECATED),
        args: m.trailing(),
    })
}

// --- add_subparser ---------------------------------------------------------

mod add_subparser {
    pub const SUBPARSERID: usize = 0;
    pub const DEST: usize = 1;
    pub const REQUIRED: usize = 2;
    pub const HELPTEXT: usize = 3;
    pub const METAVAR: usize = 4;
    pub const SUBCOMMAND: usize = 5;
    pub const PARENT_SUBPARSERID: usize = 6;
}

static ADD_SUBPARSER: SubSpec = {
    use add_subparser::*;
    SubSpec {
        options: &[
            spec("subparserid", 0, Kind::Opt),
            spec("dest", b'd', Kind::Opt),
            spec("required", b'r', Kind::Flag),
            spec("helptext", 0, Kind::Opt),
            spec("metavar", b'm', Kind::Opt),
            spec("subcommand", 0, Kind::Opt),
            spec("parent-subparserid", 0, Kind::Opt),
        ],
        positional: Positional::Name,
        conflicts: &[],
        requires: &[(PARENT_SUBPARSERID, SUBCOMMAND)],
    }
};

fn cmd_add_subparser(args: &[&str]) -> Command {
    use add_subparser::*;
    let mut m = parse_sub(&ADD_SUBPARSER, args);
    Command::AddSubparser(AddSubparserCommand {
        subparserid: m.opt(SUBPARSERID),
        name: m.positionals.pop().unwrap(),
        dest: m.opt(DEST),
        required: m.flag(REQUIRED),
        helptext: m.opt(HELPTEXT),
        metavar: m.opt(METAVAR),
        subcommand: m.opt(SUBCOMMAND),
        parent_subparserid: m.opt(PARENT_SUBPARSERID),
    })
}

// --- add_subcommand --------------------------------------------------------

mod add_subcommand {
    pub const SUBPARSERID: usize = 0;
    pub const HELPTEXT: usize = 1;
}

static ADD_SUBCOMMAND: SubSpec = SubSpec {
    options: &[spec("subparserid", 0, Kind::Opt), spec("helptext", 0, Kind::Opt)],
    positional: Positional::Name,
    conflicts: &[],
    requires: &[],
};

fn cmd_add_subcommand(args: &[&str]) -> Command {
    use add_subcommand::*;
    let mut m = parse_sub(&ADD_SUBCOMMAND, args);
    Command::AddSubcommand(AddSubcommandCommand {
        subparserid: m.opt(SUBPARSERID),
        name: m.positionals.pop().unwrap(),
        helptext: m.opt(HELPTEXT),
    })
}

// --- set_defaults ----------------------------------------------------------

mod set_defaults {
    pub const SUBCOMMAND: usize = 0;
    pub const SUBPARSERID: usize = 1;
}

static SET_DEFAULTS: SubSpec = SubSpec {
    options: &[spec("subcommand", 0, Kind::Opt), spec("subparserid", 0, Kind::Opt)],
    positional: Positional::Trailing,
    conflicts: &[],
    requires: &[],
};

fn cmd_set_defaults(args: &[&str]) -> Command {
    use set_defaults::*;
    let mut m = parse_sub(&SET_DEFAULTS, args);
    Command::SetDefaults {
        subcommand: m.opt(SUBCOMMAND),
        subparserid: m.opt(SUBPARSERID),
        args: m.trailing(),
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Same output as `urlencoding::encode_binary`: everything except
/// `[A-Za-z0-9-._~]` becomes `%XX` (uppercase hex).
fn urlencode_into(data: &[u8], out: &mut Vec<u8>) {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    for &b in data {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') {
            out.push(b);
        } else {
            out.extend_from_slice(&[b'%', HEX[(b >> 4) as usize], HEX[(b & 0xf) as usize]]);
        }
    }
}

#[no_mangle]
extern "C" fn main(argc: c_int, argv: *const *const c_char) -> c_int {
    let args: Vec<&str> = (1..argc as usize)
        .map(|i| {
            let arg = unsafe { CStr::from_ptr(*argv.add(i)) };
            match arg.to_str() {
                Ok(s) => s,
                Err(_) => die(&["arguments must be valid UTF-8"]),
            }
        })
        .collect();

    let Some((&sub, rest)) = args.split_first() else {
        die(&["a subcommand is required (new, add_arg, add_subparser, add_subcommand, set_defaults)"]);
    };
    let cmd = match sub {
        "new" => cmd_new(rest),
        "add_arg" => cmd_add_arg(rest),
        "add_subparser" => cmd_add_subparser(rest),
        "add_subcommand" => cmd_add_subcommand(rest),
        "set_defaults" => cmd_set_defaults(rest),
        "parse" => die(&["argparsh-nostd-demo does not support `parse`; use `argparsh parse`"]),
        "-h" | "--help" | "help" => no_help(),
        _ => die(&["unrecognized subcommand '", sub, "'"]),
    };

    let encoded = bitcode::encode(&cmd);
    let mut out = Vec::with_capacity(1 + encoded.len() * 3);
    out.push(DELIMITER);
    urlencode_into(&encoded, &mut out);
    if write_all(1, &out) {
        0
    } else {
        1
    }
}

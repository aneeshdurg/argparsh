//! The `#![no_std]` front door for argparsh's parser-construction commands.
//!
//! This binary supports `new`, `add_arg`, `add_subparser`, `add_subcommand`
//! and `set_defaults`, and emits exactly the same `&<urlencoded bitcode>`
//! chunks as `argparsh`, so its output can be fed to `argparsh parse`.
//! Commands that need parsing or help are delegated to the adjacent
//! `argparsh-heavy` executable.
//!
//! There is no C runtime at all here: `_start` below is the real ELF entry
//! point, every syscall is issued directly with inline `syscall`, and the
//! heap is a single 4KiB arena obtained from the kernel with one `brk` call
//! and then bump-allocated (see [`sys`] and [`BumpAlloc`]).
//!
//! Command-line handling mirrors the clap configuration in argparsh's
//! `src/main.rs`; the data types below must stay field-for-field identical to
//! the ones there, since bitcode's encoding depends on their exact shape.
#![no_std]
#![no_main]

#[cfg(not(target_arch = "x86_64"))]
compile_error!("the hand-rolled _start/syscalls here are x86_64 Linux only");

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use bitcode::Encode;
use core::alloc::{GlobalAlloc, Layout};
use core::ffi::{c_char, c_int, CStr};

const DELIMITER: u8 = b'&';
static mut ORIGINAL_ARGC: c_int = 0;
static mut ORIGINAL_ARGV: *const *const c_char = core::ptr::null();
static mut ORIGINAL_ENVP: *const *const c_char = core::ptr::null();

// ---------------------------------------------------------------------------
// Raw syscalls (x86_64 Linux calling convention: number in rax, args in rdi,
// rsi, rdx, r10, r8, r9; syscall clobbers rcx and r11; result in rax, with
// negative values in `-errno..0` signaling failure).
// ---------------------------------------------------------------------------

mod sys {
    use core::arch::asm;
    use core::ffi::c_char;

    const SYS_WRITE: i64 = 1;
    const SYS_BRK: i64 = 12;
    const SYS_EXECVE: i64 = 59;
    const SYS_EXIT_GROUP: i64 = 231;
    const SYS_READLINK: i64 = 89;

    pub const EINTR: i64 = 4;

    #[inline(always)]
    unsafe fn syscall1(n: i64, a1: i64) -> i64 {
        let ret;
        asm!(
            "syscall",
            inlateout("rax") n => ret,
            in("rdi") a1,
            out("rcx") _,
            out("r11") _,
            options(nostack),
        );
        ret
    }

    #[inline(always)]
    unsafe fn syscall3(n: i64, a1: i64, a2: i64, a3: i64) -> i64 {
        let ret;
        asm!(
            "syscall",
            inlateout("rax") n => ret,
            in("rdi") a1,
            in("rsi") a2,
            in("rdx") a3,
            out("rcx") _,
            out("r11") _,
            options(nostack),
        );
        ret
    }

    pub unsafe fn write(fd: i32, buf: *const u8, len: usize) -> i64 {
        syscall3(SYS_WRITE, fd as i64, buf as i64, len as i64)
    }

    /// `addr = 0` queries the current break instead of moving it.
    pub unsafe fn brk(addr: usize) -> usize {
        syscall1(SYS_BRK, addr as i64) as usize
    }

    pub unsafe fn readlink(path: *const u8, buf: *mut u8, bufsiz: usize) -> i64 {
        syscall3(SYS_READLINK, path as i64, buf as i64, bufsiz as i64)
    }

    pub unsafe fn execve(
        path: *const u8,
        argv: *const *const c_char,
        envp: *const *const c_char,
    ) -> i64 {
        syscall3(SYS_EXECVE, path as i64, argv as i64, envp as i64)
    }

    pub fn exit_group(code: i32) -> ! {
        unsafe {
            syscall1(SYS_EXIT_GROUP, code as i64);
        }
        // Unreachable: exit_group never returns. Loop instead of claiming UB.
        loop {
            core::hint::spin_loop();
        }
    }
}

// ---------------------------------------------------------------------------
// Entry point: the kernel jumps straight here with no C runtime behind us.
// At process start %rsp points at argc, followed by argv[0..argc], a NULL,
// envp[0..], a NULL, and the aux vector (which we don't need).
// ---------------------------------------------------------------------------

core::arch::global_asm!(
    ".global _start",
    "_start:",
    "xor ebp, ebp",   // mark the deepest frame, by convention
    "mov rdi, rsp",   // rdi = &argc, the one argument rust_entry wants
    "and rsp, -16",   // satisfy the ABI's 16-byte alignment before `call`
    "call rust_entry",
    "ud2",            // rust_entry never returns
);

#[no_mangle]
unsafe extern "C" fn rust_entry(stack: *const usize) -> ! {
    let argc = *stack as c_int;
    let argv = stack.add(1) as *const *const c_char;
    let envp = argv.add(argc as usize + 1);

    init_arena();

    ORIGINAL_ARGC = argc;
    ORIGINAL_ARGV = argv;
    ORIGINAL_ENVP = envp;

    sys::exit_group(run(argc, argv))
}

// ---------------------------------------------------------------------------
// Runtime glue: allocator, panic handler, I/O
// ---------------------------------------------------------------------------

/// Bytes handed out so far; never reset, since this allocator never frees.
static mut ARENA_OFFSET: usize = 0;
/// Base address of the arena, filled in by `init_arena` from `brk`.
static mut ARENA_BASE: *mut u8 = core::ptr::null_mut();
const ARENA_SIZE: usize = 4096;

/// Grow the break by exactly one arena's worth of bytes. Called once, before
/// any allocation can happen.
unsafe fn init_arena() {
    let base = sys::brk(0);
    let grown = sys::brk(base + ARENA_SIZE);
    if grown < base + ARENA_SIZE {
        write_all(2, b"argparsh: brk failed to grow the heap\n");
        sys::exit_group(1);
    }
    ARENA_BASE = base as *mut u8;
}

struct BumpAlloc;

unsafe impl GlobalAlloc for BumpAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let align = layout.align();
        let aligned = (ARENA_OFFSET + align - 1) & !(align - 1);
        let end = match aligned.checked_add(layout.size()) {
            Some(end) => end,
            None => return core::ptr::null_mut(),
        };
        if end > ARENA_SIZE {
            // Out of arena space: return null so `alloc`'s default handler aborts.
            return core::ptr::null_mut();
        }
        ARENA_OFFSET = end;
        ARENA_BASE.add(aligned)
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // No free: the arena is reclaimed in one shot when the process exits.
    }
}

#[global_allocator]
static ALLOCATOR: BumpAlloc = BumpAlloc;

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    write_all(2, b"argparsh: internal error\n");
    // Mimics abort()'s conventional 128+SIGABRT exit status without actually
    // raising a signal; nothing here needs a core dump.
    sys::exit_group(134)
}

/// The precompiled `alloc` crate references this symbol from its unwind tables,
/// and unoptimized builds keep that reference alive. We build with
/// `panic = "abort"` so nothing ever unwinds and it is never called.
#[no_mangle]
extern "C" fn rust_eh_personality() {}

// LLVM lowers struct copies, slice comparisons, etc. to calls to these; a C
// library normally supplies them, but we're not linking one.
use core::ffi::c_void;

#[no_mangle]
unsafe extern "C" fn memcpy(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void {
    let (dest, src) = (dest as *mut u8, src as *const u8);
    for i in 0..n {
        *dest.add(i) = *src.add(i);
    }
    dest as *mut c_void
}

#[no_mangle]
unsafe extern "C" fn memmove(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void {
    let (dest, src) = (dest as *mut u8, src as *const u8);
    if (dest as usize) < (src as usize) {
        for i in 0..n {
            *dest.add(i) = *src.add(i);
        }
    } else {
        for i in (0..n).rev() {
            *dest.add(i) = *src.add(i);
        }
    }
    dest as *mut c_void
}

#[no_mangle]
unsafe extern "C" fn memset(dest: *mut c_void, c: i32, n: usize) -> *mut c_void {
    let dest = dest as *mut u8;
    for i in 0..n {
        *dest.add(i) = c as u8;
    }
    dest as *mut c_void
}

#[no_mangle]
unsafe extern "C" fn memcmp(a: *const c_void, b: *const c_void, n: usize) -> i32 {
    let (a, b) = (a as *const u8, b as *const u8);
    for i in 0..n {
        let (x, y) = (*a.add(i), *b.add(i));
        if x != y {
            return x as i32 - y as i32;
        }
    }
    0
}

#[no_mangle]
unsafe extern "C" fn bcmp(a: *const c_void, b: *const c_void, n: usize) -> i32 {
    memcmp(a, b, n)
}

// `core::ffi::CStr::from_ptr` calls out to this rather than scanning itself.
#[no_mangle]
unsafe extern "C" fn strlen(s: *const c_char) -> usize {
    let mut n = 0;
    while *s.add(n) != 0 {
        n += 1;
    }
    n
}

fn write_all(fd: c_int, mut buf: &[u8]) -> bool {
    while !buf.is_empty() {
        let n = unsafe { sys::write(fd, buf.as_ptr(), buf.len()) };
        if n < 0 {
            if -n == sys::EINTR {
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
    sys::exit_group(2)
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

/// Find the value of `PATH` in the environment block captured at startup.
fn find_path_env() -> Option<&'static [u8]> {
    unsafe {
        let mut p = ORIGINAL_ENVP;
        if p.is_null() {
            return None;
        }
        loop {
            let entry = *p;
            if entry.is_null() {
                return None;
            }
            let bytes = CStr::from_ptr(entry).to_bytes();
            if let Some(rest) = bytes.strip_prefix(b"PATH=") {
                return Some(rest);
            }
            p = p.add(1);
        }
    }
}

/// Replace this process with the full CLI next to the front door executable.
/// `/proc/self/exe` gives the real executable path even when argparsh was
/// invoked through PATH or a symlink. PATH lookup is retained as a fallback.
fn exec_heavy(argc: c_int, argv: *const *const c_char) -> ! {
    // The arena is only 4KiB total, shared with argv/path-search buffers, so
    // this can't be PATH_MAX-sized like a libc build would make it.
    let mut executable = alloc::vec![0u8; 512];
    let proc_exe = b"/proc/self/exe\0";
    let len = unsafe { sys::readlink(proc_exe.as_ptr(), executable.as_mut_ptr(), executable.len()) };

    let mut heavy_path = if len > 0 && (len as usize) < executable.len() {
        executable.truncate(len as usize);
        match executable.iter().rposition(|b| *b == b'/') {
            Some(slash) => {
                executable.truncate(slash + 1);
                executable
            }
            None => Vec::new(),
        }
    } else {
        Vec::new()
    };
    heavy_path.extend_from_slice(b"argparsh-heavy");
    heavy_path.push(0);

    let mut heavy_argv = Vec::with_capacity(argc as usize + 1);
    // Preserve the front-door name in usage text when dispatching through
    // argparsh; direct calls to argparsh-heavy keep their own name.
    heavy_argv.push(unsafe { *argv });
    for i in 1..argc as usize {
        heavy_argv.push(unsafe { *argv.add(i) });
    }
    heavy_argv.push(core::ptr::null());

    let envp = unsafe { ORIGINAL_ENVP };

    // Try the sibling path first, then search $PATH ourselves (there's no
    // execvp without libc) so installations that split the binaries across
    // PATH entries still work.
    unsafe {
        sys::execve(heavy_path.as_ptr(), heavy_argv.as_ptr(), envp);
    }
    if let Some(path_var) = find_path_env() {
        for dir in path_var.split(|&b| b == b':') {
            let mut candidate = Vec::with_capacity(dir.len() + 16);
            if dir.is_empty() {
                candidate.push(b'.');
            } else {
                candidate.extend_from_slice(dir);
            }
            candidate.push(b'/');
            candidate.extend_from_slice(b"argparsh-heavy");
            candidate.push(0);
            unsafe {
                sys::execve(candidate.as_ptr(), heavy_argv.as_ptr(), envp);
            }
        }
    }
    write_all(2, b"argparsh: could not execute argparsh-heavy\n");
    sys::exit_group(127)
}

fn no_help() -> ! {
    unsafe { exec_heavy(ORIGINAL_ARGC, ORIGINAL_ARGV) }
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

fn run(argc: c_int, argv: *const *const c_char) -> c_int {
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
        exec_heavy(argc, argv);
    };
    if sub == "parse"
        || sub == "help"
        || !matches!(
            sub,
            "new" | "add_arg" | "add_subparser" | "add_subcommand" | "set_defaults"
        )
        || rest.iter().any(|arg| matches!(*arg, "-h" | "--help"))
    {
        exec_heavy(argc, argv);
    }
    let cmd = match sub {
        "new" => cmd_new(rest),
        "add_arg" => cmd_add_arg(rest),
        "add_subparser" => cmd_add_subparser(rest),
        "add_subcommand" => cmd_add_subcommand(rest),
        "set_defaults" => cmd_set_defaults(rest),
        "-h" | "--help" => exec_heavy(argc, argv),
        _ => unreachable!(),
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

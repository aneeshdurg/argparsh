"""Differential tests: argparsh-nostd-demo must emit exactly what argparsh does.

The binary is located via $ARGPARSH_NOSTD, falling back to argparsh-nostd-demo
on $PATH; the tests are skipped if neither is available.
"""

import json
import os
import shutil
import subprocess

import pytest

NOSTD = os.environ.get("ARGPARSH_NOSTD") or shutil.which("argparsh-nostd-demo")
pytestmark = pytest.mark.skipif(NOSTD is None, reason="argparsh-nostd-demo not found")


def run(binary, args):
    return subprocess.run([binary, *args], capture_output=True)


# Invocations that argparsh accepts; output must be byte-identical.
ACCEPTED = [
    ["new", "prog"],
    ["new", "prog", "-d", "desc", "-e", "bye!"],
    ["new", "prog", "--description=desc", "--epilog", ""],
    ["new", "-dfoo", "prog"],
    ["new", "-d=foo", "prog"],
    ["new", "prog", "-d="],
    ["new", "--", "-prog"],
    ["new", "-"],
    ["new", "ünïcödé", "-d", "spaces & symbols %20 ~._-"],
    ["add_arg", "x"],
    ["add_arg", "-i", "--interval"],
    ["add_arg", "--type", "int", "--default", "10", "--", "-i", "--interval"],
    ["add_arg", "--action", "store_true", "--", "-f"],
    ["add_arg", "--choice", "a", "--choice", "b", "-c", "c", "--helptext", "h", "--", "a"],
    ["add_arg", "-ca", "-cb", "x"],
    ["add_arg", "-rd1", "x"],
    ["add_arg", "-rz"],
    ["add_arg", "-V"],
    ["add_arg", "-"],
    ["add_arg", ""],
    ["add_arg", "--"],
    ["add_arg", "--", "--", "y"],
    ["add_arg", "x", "--", "y"],
    ["add_arg", "-r", "x", "-r"],
    ["add_arg", "-r", "--", "-r"],
    ["add_arg", "--type", "int", "x", "-i"],
    ["add_arg", "-d", "-5", "x"],
    ["add_arg", "-d", "-", "x"],
    ["add_arg", "--default", "--x", "x"],
    ["add_arg", "--unknown", "x"],
    ["add_arg", "--=x"],
    ["add_arg", "--store-const="],
    ["add_arg", "--append-const", "k", "x"],
    ["add_arg", "-n", "01", "x"],
    ["add_arg", "-n", "+3", "x"],
    ["add_arg", "--nargs", "+", "x"],
    ["add_arg", "--nargs", "*", "x"],
    ["add_arg", "-a", "count", "-v"],
    ["add_arg", "-a", "append", "-a2"],
    ["add_arg", "--action=help", "--", "-h"],
    ["add_arg", "--displays-version", "--version", "1.0", "--", "-v"],
    ["add_arg", "--subcommand", "foo", "--subparserid", "foobar", "qux"],
    ["add_arg", "--metavar", "M", "--dest", "D", "--deprecated", "--required", "x"],
    ["add_arg", "-t", "float", "-c", "1.5", "x"],
    ["add_subparser", "foobar"],
    ["add_subparser", "foobar", "--required"],
    ["add_subparser", "y", "-m", "M", "-d", "D", "-r", "--helptext", "H", "--subparserid", "I"],
    ["add_subparser", "--subcommand", "s", "--parent-subparserid", "x", "y"],
    ["add_subparser", "--", "-a"],
    ["add_subparser", "-rdD", "y"],
    ["add_subcommand", "foo"],
    ["add_subcommand", "y", "--subparserid", "I", "--helptext", "H"],
    ["add_subcommand", "y", "--helptext=H"],
    ["set_defaults"],
    ["set_defaults", "--subcommand", "fee", "--myfooarg", "fee"],
    ["set_defaults", "--subcommand=fee", "--", "--x", "y"],
    ["set_defaults", "--x", "--subcommand", "fee"],
    ["set_defaults", "--subparserid", "a", "-k", "v"],
]

# Invocations that argparsh rejects; both must fail with status 2 and print
# nothing to stdout.
REJECTED = [
    [],
    ["bogus"],
    ["new"],
    ["new", "a", "b"],
    ["new", "a", "-d"],
    ["new", "a", "-d", "x", "-d", "y"],
    ["new", "a", "-d", "-5"],
    ["new", "a", "-d", "--"],
    ["new", "a", "-d", "--", "x"],
    ["new", "a", "--", "b"],
    ["new", "-V"],
    ["new", "a", "--bogus"],
    ["add_arg", "--required", "--required", "x"],
    ["add_arg", "--required=true", "x"],
    ["add_arg", "-d", "1", "-d", "2", "x"],
    ["add_arg", "-d"],
    ["add_arg", "-d", "--required", "x"],
    ["add_arg", "-d", "-r", "x"],
    ["add_arg", "-d", "--", "x"],
    ["add_arg", "--choice"],
    ["add_arg", "-rda"],
    ["add_arg", "-n", "-1", "x"],
    ["add_arg", "-n", "x", "y"],
    ["add_arg", "--nargs", "?", "x"],
    ["add_arg", "-a", "Store", "x"],
    ["add_arg", "--nargs", "+", "--nargs-exact", "2"],
    ["add_arg", "-a", "store_true", "--store-const", "1"],
    ["add_arg", "--store-const", "1", "--append-const", "2"],
    ["add_arg", "--displays-version", "--version", "1", "--nargs", "+"],
    ["add_arg", "--version", "1", "x"],
    ["add_arg", "--displays-version", "x"],
    ["add_arg", "--subparserid", "a", "x"],
    ["add_arg", "-r", "-d", "1", "x"],
    ["add_subparser"],
    ["add_subparser", "-r"],
    ["add_subparser", "a", "b"],
    ["add_subparser", "--parent-subparserid", "x", "y"],
    ["add_subcommand"],
    ["add_subcommand", "a", "--bogus"],
    ["set_defaults", "--subcommand"],
]


@pytest.mark.parametrize("args", ACCEPTED, ids=lambda a: " ".join(a) or "<none>")
def test_matches_argparsh(args):
    expected = run("argparsh", args)
    assert expected.returncode == 0, expected.stderr
    actual = run(NOSTD, args)
    assert actual.returncode == 0, actual.stderr
    assert actual.stdout == expected.stdout


@pytest.mark.parametrize("args", REJECTED, ids=lambda a: " ".join(a) or "<none>")
def test_rejects_like_argparsh(args):
    expected = run("argparsh", args)
    assert expected.returncode == 2
    actual = run(NOSTD, args)
    assert actual.returncode == 2
    assert actual.stdout == b""
    assert actual.stderr != b""


@pytest.mark.parametrize("args", [["parse", "&x", "--"], ["--help"], ["add_arg", "-h"]])
def test_unsupported(args):
    actual = run(NOSTD, args)
    assert actual.returncode == 2
    assert actual.stdout == b""


def test_parse_with_argparsh():
    """A parser built by argparsh-nostd-demo is usable by `argparsh parse`."""
    commands = [
        ["new", "prog", "-d", "demo"],
        ["add_arg", "--choice", "a", "--choice", "b", "--", "letter"],
        ["add_arg", "--type", "int", "--default", "10", "--", "-i", "--interval"],
        ["add_arg", "--action", "store_true", "--", "-f"],
        ["add_subparser", "foobar", "--required"],
        ["add_subcommand", "foo"],
        ["add_subcommand", "bar"],
        ["add_arg", "--subcommand", "foo", "qux"],
        ["set_defaults", "--subcommand", "foo", "--cmd", "foo"],
    ]
    cli = ["a", "-i", "5", "-f", "foo", "Q"]

    def parse(binary):
        parser = b"".join(run(binary, args).stdout for args in commands)
        result = subprocess.run(
            ["argparsh", "parse", parser, "--format", "json", "--", *cli],
            capture_output=True,
            check=True,
        )
        return json.loads(result.stdout)

    parsed = parse(NOSTD)
    assert parsed == parse("argparsh")
    assert parsed["letter"] == "a"
    assert parsed["interval"] == 5
    assert parsed["qux"] == "Q"
    assert parsed["cmd"] == "foo"

import json
import subprocess
import textwrap


def test_noargs():
    parser = subprocess.check_output(
        [
            "argparsh",
            "new",
            "myprog",
            "--description",
            "hello myprog",
            "--epilog",
            "end of help text",
        ]
    )
    p = subprocess.Popen(
        ["argparsh", "parse", parser, "--", "-h"],
        stderr=subprocess.PIPE,
        stdout=subprocess.PIPE,
    )
    p.wait()
    stdout = p.stdout.read()
    stderr = p.stderr.read()

    assert stderr.decode() == textwrap.dedent(
        """\
            usage: myprog [-h]

            hello myprog

            options:
              -h, --help  show this help message and exit

            end of help text
        """
    )
    assert stdout == b"exit 0\n"

    p = subprocess.Popen(
        ["argparsh", "parse", parser, "--"],
        stderr=subprocess.PIPE,
        stdout=subprocess.PIPE,
    )
    stdout, stderr = p.communicate()
    assert stdout == b""
    assert stderr == b""

    p = subprocess.Popen(
        ["argparsh", "parse", parser, "--", "foobar"],
        stderr=subprocess.PIPE,
        stdout=subprocess.PIPE,
    )
    stdout, stderr = p.communicate()
    assert stdout == b"exit 2\n"
    assert p.returncode == 2
    assert stderr.decode() == textwrap.dedent(
        """\
            usage: myprog [-h]
            myprog: error: unrecognized arguments: foobar
        """
    )


def test_single_arg():
    parser = subprocess.check_output(["argparsh", "new", "myprog"])
    parser += subprocess.check_output(
        ["argparsh", "add_arg", "--helptext", "arg0 help test", "arg0"]
    )

    p = subprocess.Popen(
        ["argparsh", "parse", parser, "--", "-h"],
        stderr=subprocess.PIPE,
        stdout=subprocess.PIPE,
    )
    stdout, stderr = p.communicate()

    assert stdout == b"exit 0\n"
    assert stderr.decode() == textwrap.dedent(
        """\
            usage: myprog [-h] arg0

            positional arguments:
              arg0        arg0 help test

            options:
              -h, --help  show this help message and exit
        """
    )

    p = subprocess.Popen(
        ["argparsh", "parse", parser, "--format", "json", "--", "VALUE '0'"],
        stderr=subprocess.PIPE,
        stdout=subprocess.PIPE,
    )
    stdout, stderr = p.communicate()
    assert stderr == b""

    kv = json.loads(stdout.decode())
    assert kv == {"arg0": "VALUE '0'"}


def test_set_defaults():
    parser = subprocess.check_output(["argparsh", "new", "myprog"])
    parser += subprocess.check_output(["argparsh", "set_defaults", "--key0", "value0"])

    parser += subprocess.check_output(["argparsh", "add_subparser", "foobar"])
    parser += subprocess.check_output(["argparsh", "add_subcommand", "foo"])
    parser += subprocess.check_output(
        ["argparsh", "set_defaults", "--subcommand", "foo", "--cmd", "foo"]
    )
    parser += subprocess.check_output(["argparsh", "add_subcommand", "bar"])
    parser += subprocess.check_output(
        ["argparsh", "set_defaults", "--subcommand", "bar", "--cmd", "bar"]
    )

    def parse_args(args):
        p = subprocess.Popen(
            ["argparsh", "parse", parser, "--format", "json", "--"] + args,
            stderr=subprocess.PIPE,
            stdout=subprocess.PIPE,
        )
        stdout, stderr = p.communicate()
        assert stderr == b""
        return json.loads(stdout.decode())

    assert parse_args([]) == {"foobar": None, "key0": "value0"}
    assert parse_args(["foo"]) == {"foobar": "foo", "key0": "value0", "cmd": "foo"}
    assert parse_args(["bar"]) == {"foobar": "bar", "key0": "value0", "cmd": "bar"}


def test_multiple_subparsers():
    # This is the example from the help text
    cmds = textwrap.dedent(
        """
        # Create two subcommands `<prog> foo` and `<prog> bar`
        argparsh add_subparser foobar --required
        argparsh add_subcommand foo
        argparsh add_subcommand bar

        # Attach a subcommand to `foo`, creating
        #    <prog> foo fee
        # -and-
        #    <prog> foo fie
        argparsh add_subparser feefie --subcommand foo --required
        argparsh add_subcommand fee
        argparsh set_defaults --subcommand fee --myfooarg fee
        argparsh add_subcommand fie
        argparsh set_defaults --subcommand fie --myfooarg fie

        # Add a regular argument to foo. Note that we now need to
        # use the metaname "foobar" so avoid attaching to the wrong
        # parser. (By default the most recently created parser is
        # used - in this case the most recently created parser is
        # feefie)
        argparsh add_arg --subparserid foobar --subcommand foo qux
        argparsh set_defaults --subparserid foobar --subcommand foo --myarg foo

        # Attach a regular argument to bar
        argparsh add_arg --subparserid foobar --subcommand bar baz
        argparsh set_defaults --subparserid foobar --subcommand bar --myarg bar

        # possible commands supported by this parser:
        #   <prog> foo fee <qux>
        #   <prog> foo fie <qux>
        #   <prog> bar <baz>
    """
    )
    cmds = [l.split() for l in cmds.split("\n") if len(l) and not l.startswith("#")]
    parser = b""
    for cmd in cmds:
        parser += subprocess.check_output(cmd)

    def parse_args(args):
        p = subprocess.Popen(
            ["argparsh", "parse", parser, "--format", "json", "--"] + args,
            stderr=subprocess.PIPE,
            stdout=subprocess.PIPE,
        )
        stdout, stderr = p.communicate()
        assert stderr == b""
        return json.loads(stdout.decode())

    assert parse_args(["foo", "fee", "qux0"]) == {
        "feefie": "fee",
        "foobar": "foo",
        "myarg": "foo",
        "myfooarg": "fee",
        "qux": "qux0",
    }
    assert parse_args(["foo", "fie", "qux1"]) == {
        "feefie": "fie",
        "foobar": "foo",
        "myarg": "foo",
        "myfooarg": "fie",
        "qux": "qux1",
    }
    assert parse_args(["bar", "baz0"]) == {
        "foobar": "bar",
        "myarg": "bar",
        "baz": "baz0",
    }

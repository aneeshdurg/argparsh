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
    stdout, stderr = p.communicate()

    assert stdout == b"exit 0\n"
    assert stderr.decode() == textwrap.dedent(
        """\
            usage: myprog [-h]

            hello myprog

            options:
              -h, --help  show this help message and exit

            end of help text
        """
    )

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
    assert stderr.decode() == textwrap.dedent(
        """\
            usage: myprog [-h]
            myprog: error: unrecognized arguments: foobar
        """
    )


def test_simple():
    parser = subprocess.check_output(["argparsh", "new", "myprog"])
    parser += subprocess.check_output(
        ["argparsh", "add_arg", "arg0", "--", "--help", "arg0 help test"]
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
        ["argparsh", "parse", parser, "--", "VALUE '0'"],
        stderr=subprocess.PIPE,
        stdout=subprocess.PIPE,
    )
    stdout, stderr = p.communicate()

    assert stdout == b"""arg0="VALUE '0'"\n"""
    assert stderr == b""

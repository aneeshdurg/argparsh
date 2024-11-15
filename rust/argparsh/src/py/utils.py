import argparse
from dataclasses import dataclass, field
import uuid
import sys
import json


def arglist_to_kwargs(arglist):
    kwargs = {}
    for i in range(0, len(arglist), 2):
        assert arglist[i].startswith("--")
        key = arglist[i][2:]
        value = arglist[i + 1]
        if key in ["type", "choices"]:
            kwargs[key] = eval(value, {}, {})
        elif key == "nargs":
            try:
                kwargs[key] = int(value)
            except ValueError:
                kwargs[key] = value
        elif key in ["deprecated", "required"]:
            kwargs[key] = bool(value)
        else:
            kwargs[key] = value
    return kwargs


@dataclass
class Parser:
    _parser: argparse.ArgumentParser | None = None
    _subparsers: dict[str, argparse._SubParsersAction] = field(default_factory=dict)
    _parsers: dict[str, dict[str, argparse.ArgumentParser]] = field(
        default_factory=dict
    )

    def initialize(self, *args, **kwargs):
        self._parser = argparse.ArgumentParser(
            *args, **kwargs, formatter_class=argparse.RawTextHelpFormatter
        )

    @property
    def parser(self) -> argparse.ArgumentParser:
        if self._parser is None:
            self._parser = argparse.ArgumentParser()
        return self._parser

    def add_subparser(self, subparser, parser_arg, metaname, **kwargs):
        if metaname is None:
            metaname = str(uuid.uuid4())
        p = self.get_parser(parser_arg, subparser)
        self._subparsers[metaname] = p.add_subparsers(**kwargs)

    def add_parser(self, metaname, name, **kwargs):
        if metaname is None:
            metaname = list(self._subparsers.keys())[-1]
        if metaname not in self._parsers:
            self._parsers[metaname] = {}
        self._parsers[metaname][name] = self._subparsers[metaname].add_parser(
            name, **kwargs
        )

    def get_parser(self, metaname, name):
        if metaname is None and name is None:
            return self.parser

        if metaname is None:
            metaname = list(self._subparsers.keys())[-1]
        assert (
            metaname in self._subparsers
        ), f"Could not find parser with name {metaname}"
        assert (
            name in self._parsers[metaname]
        ), f"Could not find subparser with name {name}"
        return self._parsers[metaname][name]

    def add_argument(self, args, subparser, parser_arg):
        # add an argument to obj by assembling the method to call
        aliases = []
        while len(args) and not args[0] == "--":
            aliases.append(args[0])
            args.pop(0)
        meth_args = aliases

        if len(args):
            args.pop(0)

        meth_kwargs = arglist_to_kwargs(args)
        p = self.get_parser(parser_arg, subparser)
        p.add_argument(*meth_args, **meth_kwargs)


_output_format = {}


def output_format(name: str):
    def deco(f):
        _output_format[name] = f
        return f

    return deco


@output_format("shell")
def output_shell(kv: dict, extra_args: list[str], output):
    parser = argparse.ArgumentParser(
        "argparsh parser --format shell",
        description="Declare a variable for every CLI argument",
    )
    parser.add_argument(
        "-p", "--prefix", help="Prefix to add to every declared variable", default=""
    )
    parser.add_argument(
        "-e",
        "--export",
        action="store_true",
        help="Export declarations to the environment",
    )
    parser.add_argument(
        "-l", "--local", action="store_true", help="declare variable as local"
    )
    args = parser.parse_args(extra_args)

    assert not (
        args.local and args.export
    ), "args cannot be declared as both local and export"
    export = ""
    if args.export:
        export = "export "
    if args.local:
        export = "local "

    for k, v in kv.items():
        print(f"{export}{args.prefix}{k}={repr(v)}", file=output)


@output_format("assoc_array")
def output_assoc_array(kv: dict, extra_args: list[str], output):
    parser = argparse.ArgumentParser(
        "argparsh parser --format assoc_array",
        description="Create an associative array from parsed arguments",
    )
    parser.add_argument(
        "-n", "--name", required=True, help="Name of variable to output into"
    )
    args = parser.parse_args(extra_args)

    print(f"declare -A {args.name}", file=output)
    for k, v in kv.items():
        print(f'{args.name}["{k}"]={repr(v)}', file=output)


@output_format("json")
def output_json(kv: dict, extra_args: list[str], output):
    assert len(extra_args) == 0
    json.dump(kv, output, indent=4)


def run_parse(parser: Parser, format: str, args: list[str]):
    output = sys.stdout
    sys.stdout = sys.stderr

    extra_args = []
    found_sep = False
    while len(args):
        if args[0] == "--":
            args.pop(0)
            found_sep = True
            break
        extra_args.append(args[0])
        args.pop(0)
    if not found_sep:
        args = extra_args
        extra_args = []

    try:
        parsed_args = parser.parser.parse_args(args)
        _output_format[format](dict(parsed_args._get_kwargs()), extra_args, output)
    except SystemExit as e:
        print(f"exit {e}", file=output)
        exit(e.code)

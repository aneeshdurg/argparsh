import argparse
from dataclasses import dataclass, field
import sys
import json


def arglist_to_kwargs(arglist):
    kwargs = {}
    if arglist == None:
        return kwargs
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


def add_if_present(kwargs, cmd, name, rename=None):
    v = getattr(cmd, name)
    if v is None:
        return
    kwargs[rename or name] = v


def get_action_str(action):
    action_to_str = {
        str(Action.Store): "store",
        str(Action.StoreTrue): "store_true",
        str(Action.Append): "append",
        str(Action.Count): "count",
        str(Action.Help): "help",
    }
    return action_to_str[str(action)]


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

    def add_parser(self, subparserid, name, kwargs):
        if subparserid is None:
            subparserid = list(self._subparsers.keys())[-1]
        if subparserid not in self._parsers:
            self._parsers[subparserid] = {}
        self._parsers[subparserid][name] = self._subparsers[subparserid].add_parser(
            name, **kwargs
        )

    def get_parser(self, subparserid, name):
        if subparserid is None and name is None:
            return self.parser

        if subparserid is None:
            subparserid = list(self._subparsers.keys())[-1]
        assert (
            subparserid in self._subparsers
        ), f"Could not find parser with name {subparserid}"
        assert (
            name in self._parsers[subparserid]
        ), f"Could not find subparser with name {name}"
        return self._parsers[subparserid][name]

    def cmd_add_argument(self, cmd):
        args = cmd.args if cmd.args else []
        kwargs = {}
        # TODO: ???
        # add_if_present("deprecated")
        add_if_present(kwargs, cmd, "dest")
        add_if_present(kwargs, cmd, "helptext", "help")
        add_if_present(kwargs, cmd, "metavar")

        if cmd.required:
            kwargs["required"] = True

        type_ = str
        if cmd.type_ is not None:
            type_ = eval(cmd.type_, {}, {})
            kwargs["type"] = type_

        if cmd.choice is not None:
            # If we have a type, then cast the choices to that type
            kwargs["choices"] = [type_(c) for c in cmd.choice]

        if cmd.default is not None:
            # If we have a type, then cast the default to that type
            kwargs["default"] = type_(cmd.default)

        if cmd.action is not None:
            kwargs["action"] = get_action_str(cmd.action)
        elif cmd.store_const is not None:
            kwargs["action"] = "store_const"
            kwargs["const"] = type_(cmd.store_const)
            if "type" in kwargs:
                del kwargs["type"]
        elif cmd.append_const is not None:
            kwargs["action"] = "append_const"
            kwargs["const"] = type_(cmd.append_const)
            if "type" in kwargs:
                del kwargs["type"]

        if cmd.displays_version:
            kwargs["action"] = "version"
            add_if_present(kwargs, cmd, "version")

        if cmd.nargs is not None:
            if cmd.nargs == NArgs.AtLeastOne:
                kwargs["nargs"] = "+"
            else:
                kwargs["nargs"] = "*"
        elif cmd.nargs_exact is not None:
            kwargs["nwargs"] = cmd.nargs_exact

        p = self.get_parser(cmd.subparserid, cmd.subcommand)
        p.add_argument(*args, **kwargs)

    def cmd_add_subparser(self, cmd):
        p = self.get_parser(cmd.parent_subparserid, cmd.subcommand)

        kwargs = {}
        kwargs["title"] = cmd.name
        kwargs["dest"] = cmd.name
        add_if_present(kwargs, cmd, "dest")
        add_if_present(kwargs, cmd, "helptext", rename="help")
        add_if_present(kwargs, cmd, "metavar")
        if cmd.required:
            kwargs["required"] = True

        subparserid = cmd.subparserid if cmd.subparserid is not None else cmd.name
        self._subparsers[subparserid] = p.add_subparsers(**kwargs)

    def cmd_add_subcommand(self, cmd):
        subparserid = (
            cmd.subparserid
            if cmd.subparserid is not None
            else list(self._subparsers.keys())[-1]
        )
        if subparserid not in self._parsers:
            self._parsers[subparserid] = {}

        kwargs = {}
        add_if_present(kwargs, cmd, "helptext", "help")
        self._parsers[subparserid][cmd.name] = self._subparsers[subparserid].add_parser(
            cmd.name, **kwargs
        )

    def cmd_set_defaults(self, subparser, subparserid, args):
        kwargs = arglist_to_kwargs(args)
        p = self.get_parser(subparserid, subparser)
        p.set_defaults(**kwargs)


_output_format = {}


def output_format(name: str):
    def deco(f):
        _output_format[name] = f
        return f

    return deco


def dump(value):
    if isinstance(value, (int, float, str)):
        return repr(value)
    return repr(json.dumps(value))


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
        v = dump(v)
        print(f"{export}{args.prefix}{k}={v}", file=output)


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


def run_parse(parser: Parser, format: str, args: list[str] | None):
    if args is None:
        args = []
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
    finally:
        output.flush()

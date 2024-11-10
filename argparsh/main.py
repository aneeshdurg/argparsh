import argparse
import sys
from abc import abstractmethod

from . import utils

commands: dict[str, "Command"] = {}


class Watcher(type):
    """Register all subclasses into the commands global dictionary by their name"""

    def __init__(cls, name, bases, clsdict):
        if len(cls.mro()) > 2:
            commands[cls.name()] = cls
        super(Watcher, cls).__init__(name, bases, clsdict)


class Command(metaclass=Watcher):
    @classmethod
    @abstractmethod
    def name(cls) -> str:
        pass

    @classmethod
    @abstractmethod
    def help(cls) -> str:
        pass

    @classmethod
    def requires_subparser_arg(cls) -> bool:
        return False

    @classmethod
    def consumes_rest_args(cls) -> bool:
        return False

    @classmethod
    def extend_parser(cls, parser: argparse.ArgumentParser):
        pass

    @classmethod
    @abstractmethod
    def construct(cls, args: argparse.Namespace) -> str:
        pass

    @classmethod
    @abstractmethod
    def run(cls, parser: utils.Parser, data):
        pass

    @classmethod
    def output(cls, data) -> str:
        return utils.output(cls.name(), data)


class New(Command):
    @classmethod
    def name(cls):
        return "new"

    @classmethod
    def help(cls):
        return "Create a new parser with a name and description"

    @classmethod
    def extend_parser(cls, parser: argparse.ArgumentParser):
        parser.add_argument("name", help="Name of script")
        parser.add_argument(
            "-d",
            "--description",
            help="Description of program",
            action="store",
            default="",
        )
        parser.add_argument(
            "-e",
            "--epilog",
            help="Text to display after help text",
            action="store",
            default="",
        )

    @classmethod
    def construct(cls, args: argparse.Namespace) -> str:
        kwargs = {
            "prog": args.name,
            "description": args.description,
            "epilog": args.epilog,
        }
        return cls.output(kwargs)

    @classmethod
    def run(cls, parser: utils.Parser, data):
        parser.initialize(**data)


class AddArg(Command):
    @classmethod
    def name(cls):
        return "add_arg"

    @classmethod
    def help(cls):
        return "Add an argument to the parser (separate argument aliases and parsing options with '--' )"

    @classmethod
    def requires_subparser_arg(cls) -> bool:
        return True

    @classmethod
    def consumes_rest_args(cls) -> bool:
        return True

    @classmethod
    def construct(cls, args: argparse.Namespace) -> str:
        # add an argument to obj by assembling the method to call
        aliases = []
        while len(args.rest) and not args.rest[0] == "--":
            aliases.append(args.rest[0])
            args.rest.pop(0)
        meth_args = aliases

        if len(args.rest):
            args.rest.pop(0)

        meth_kwargs = utils.arglist_to_kwargs(args.rest)
        return cls.output((args.subparser, args.parser_arg, meth_args, meth_kwargs))

    @classmethod
    def run(cls, parser: utils.Parser, data):
        subparser, parser_arg, meth_args, meth_kwargs = data
        p = parser.get_parser(parser_arg, subparser)
        p.add_argument(*meth_args, **meth_kwargs)


class SetDefault(Command):
    @classmethod
    def name(cls):
        return "set_defaults"

    @classmethod
    def help(cls):
        return "Set defaults for parser with key/value pairs"

    @classmethod
    def requires_subparser_arg(cls) -> bool:
        return True

    @classmethod
    def consumes_rest_args(cls) -> bool:
        return True

    @classmethod
    def construct(cls, args: argparse.Namespace) -> str:
        meth_kwargs = utils.arglist_to_kwargs(args.rest)
        return cls.output((args.subparser, args.parser_arg, meth_kwargs))

    @classmethod
    def run(cls, parser: utils.Parser, data):
        subparser, parser_arg, meth_kwargs = data
        p = parser.get_parser(parser_arg, subparser)
        p.set_defaults(**meth_kwargs)


class SubparserInit(Command):
    @classmethod
    def name(cls):
        return "subparser_init"

    @classmethod
    def help(cls):
        return "Initialize a new subparser"

    @classmethod
    def consumes_rest_args(cls) -> bool:
        return True

    @classmethod
    def extend_parser(cls, parser: argparse.ArgumentParser):
        parser.add_argument(
            "--metaname",
            help="Optional name for argument",
            required=False,
            default=None,
        )

    @classmethod
    def construct(cls, args: argparse.Namespace) -> str:
        data = utils.arglist_to_kwargs(args.rest)
        return cls.output((args.metaname, data))

    @classmethod
    def run(cls, parser: utils.Parser, data):
        metaname, kwargs = data
        parser.add_subparser(metaname, **kwargs)


class SubparserAdd(Command):
    @classmethod
    def name(cls):
        return "subparser_add"

    @classmethod
    def help(cls):
        return "Add a command to a subparser"

    @classmethod
    def consumes_rest_args(cls) -> bool:
        return True

    @classmethod
    def extend_parser(cls, parser: argparse.ArgumentParser):
        parser.add_argument(
            "--metaname",
            help="Name of subparser to add to (from subparser_init)",
            required=False,
            default=None,
        )
        parser.add_argument("name", help="Name of command")

    @classmethod
    def construct(cls, args: argparse.Namespace) -> str:
        data = utils.arglist_to_kwargs(args.rest)
        return cls.output((args.name, args.metaname, data))

    @classmethod
    def run(cls, parser: utils.Parser, data):
        name, metaname, kwargs = data
        parser.add_parser(metaname, name, **kwargs)


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

    for k, v in kv:
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
    for k, v in kv:
        print(f'{args.name}["{k}"]={repr(v)}', file=output)


def main():
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(required=True)

    for command in commands.values():
        p = subparsers.add_parser(command.name(), help=command.help())
        p.set_defaults(command=command)
        if command.requires_subparser_arg():
            p.add_argument(
                "--subparser",
                help="Name of subparser command (argument to create)",
                default=None,
            )
            p.add_argument(
                "--parser-arg",
                help="Name of subparser argument (argument to init)",
                default=None,
            )
        command.extend_parser(p)

    p = subparsers.add_parser("parse", help="Parse command line arguments")
    p.set_defaults(command=None)
    p.add_argument("state", help="Parser program constructed by argparsh calls")
    p.add_argument(
        "--format",
        default="shell",
        choices=_output_format.keys(),
        help="Output format of parsed arguments",
    )

    args, unconsumed = parser.parse_known_args()
    if args.command is not None and not args.command.consumes_rest_args():
        if len(unconsumed):
            raise ValueError(f"Unexpected arguments! {unconsumed}")
    args.rest = unconsumed

    if args.command:
        print(args.command.construct(args), end="")
    else:
        output = sys.stdout
        sys.stdout = sys.stderr

        actions = utils.parse_state(args.state)

        new_parser = utils.Parser()
        for name, data in actions:
            commands[name].run(new_parser, data)

        extra_args = []
        found_sep = False
        while len(args.rest):
            if args.rest[0] == "--":
                args.rest.pop(0)
                found_sep = True
                break
            extra_args.append(args.rest[0])
            args.rest.pop(0)
        if not found_sep:
            args.rest = extra_args
            extra_args = []

        try:
            parsed_args = new_parser.parser.parse_args(args.rest)
            _output_format[args.format](parsed_args._get_kwargs(), extra_args, output)
        except SystemExit as e:
            print(f"exit {e}", file=output)
            exit(e.code)

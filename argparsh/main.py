import argparse
import sys
from abc import abstractmethod

from . import utils

commands: dict[str, "Command"] = {}


class Watcher(type):
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
    def extend_parser(cls, parser: argparse.ArgumentParser):
        parser.add_argument("name")
        parser.add_argument("-d", "--description", action="store", default="")
        parser.add_argument("-e", "--epilog", action="store", default="")

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
    def consumes_rest_args(cls) -> bool:
        return True

    @classmethod
    def extend_parser(cls, parser: argparse.ArgumentParser):
        parser.add_argument("--metaname", required=False, default=None)

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
    def consumes_rest_args(cls) -> bool:
        return True

    @classmethod
    def extend_parser(cls, parser: argparse.ArgumentParser):
        parser.add_argument("--metaname", required=False, default=None)
        parser.add_argument("name")

    @classmethod
    def construct(cls, args: argparse.Namespace) -> str:
        data = utils.arglist_to_kwargs(args.rest)
        return cls.output((args.name, args.metaname, data))

    @classmethod
    def run(cls, parser: utils.Parser, data):
        name, metaname, kwargs = data
        parser.add_parser(metaname, name, **kwargs)


def main():
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(required=True)

    for command in commands.values():
        p = subparsers.add_parser(command.name())
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

    p = subparsers.add_parser("parse")
    p.set_defaults(command=None)
    p.add_argument("state")

    args, unconsumed = parser.parse_known_args()
    if args.command is not None and not args.command.consumes_rest_args():
        if len(unconsumed):
            raise ValueError(f"Unexpected arguments! {unconsumed}")
    args.rest = unconsumed

    if args.command:
        print(args.command.construct(args), end="")
    else:
        actions = utils.parse_state(args.state)

        new_parser = utils.Parser()
        for name, data in actions:
            commands[name].run(new_parser, data)

        output = sys.stdout
        sys.stdout = sys.stderr
        try:
            parsed_args = new_parser.parser.parse_args(args.rest)
            for k, v in parsed_args._get_kwargs():
                print(f"{k.upper()}={repr(v)}", file=output)
        except SystemExit as e:
            print(f"exit {e}", file=output)
            exit(e.code)

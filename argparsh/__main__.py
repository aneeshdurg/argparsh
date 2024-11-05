import argparse
import sys
from abc import abstractmethod

from . import utils

commands:dict[str, "Command"] = {}
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
    def requires_state(cls) -> bool:
        return True

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
    def append_to_state(cls, state, data) -> str:
        return utils.append_to_state(state, cls.name(), data)

class New(Command):
    @classmethod
    def name(cls):
        return "new"

    @classmethod
    def requires_state(cls) -> bool:
        return False

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
        return cls.append_to_state("", kwargs)

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
        return cls.append_to_state(args.state, (args.subparser, args.parser_arg, meth_args, meth_kwargs))

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
        return cls.append_to_state(
            args.state, (args.subparser, args.parser_arg, meth_kwargs)
        )

    @classmethod
    def run(cls, parser: utils.Parser, data):
        subparser, parser_arg, meth_kwargs = data
        p = parser.get_parser(parser_arg, subparser)
        p.set_defaults(**meth_kwargs)


class Subparsers(Command):
    @classmethod
    def name(cls):
        return "subparser"

    @classmethod
    def extend_parser(cls, parser: argparse.ArgumentParser):
        parser.add_argument("--metaname", required=False, default=None)

        subparsers = parser.add_subparsers(required=True)

        init = subparsers.add_parser("init")
        init.set_defaults(subparser_func=cls.do_init)

        create = subparsers.add_parser("create")
        create.add_argument("name")
        create.set_defaults(subparser_func=cls.do_create)

        parser.add_argument("rest", nargs=argparse.REMAINDER)

    @classmethod
    def do_init(cls, args: argparse.Namespace):
        data = utils.arglist_to_kwargs(args.rest)
        return cls.append_to_state(args.state, ("init", (args.metaname, data)))

    @classmethod
    def do_create(cls, args: argparse.Namespace):
        data = utils.arglist_to_kwargs(args.rest)
        return cls.append_to_state(args.state, ("create", (args.name, args.metaname, data)))

    @classmethod
    def construct(cls, args: argparse.Namespace) -> str:
        return args.subparser_func(args)

    @classmethod
    def run(cls, parser: utils.Parser, data):
        subcommand, args = data
        if subcommand == "init":
            metaname, kwargs = args
            parser.add_subparser(metaname, **kwargs)
        elif subcommand == "create":
            name, metaname, kwargs = args
            parser.add_parser(metaname, name, **kwargs)


parser = argparse.ArgumentParser()
subparsers = parser.add_subparsers(required=True)

for command in commands.values():
    p = subparsers.add_parser(command.name())
    p.set_defaults(command=command)
    if command.requires_state():
        p.add_argument("state")
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
    if command.consumes_rest_args():
        p.add_argument("rest", nargs=argparse.REMAINDER)
    command.extend_parser(p)

p = subparsers.add_parser("parse")
p.set_defaults(command=None)
p.add_argument("state")
p.add_argument("rest", nargs=argparse.REMAINDER)

args = parser.parse_args()
if args.command:
    print(args.command.construct(args))
else:
    actions = utils.parse_state(args.state)

    new_parser = utils.Parser()
    for name, data in actions:
        commands[name].run(new_parser, data)

    output = sys.stdout
    sys.stdout = sys.stderr
    exit_ = None
    try:
        parsed_args = new_parser.parser.parse_args(args.rest)
        for k, v in parsed_args._get_kwargs():
            print(f"{k.upper()}={repr(v)}", file=output)
    except SystemExit as e:
        print(f"exit {e}", file=output)
        exit_ = e
    exit(0)

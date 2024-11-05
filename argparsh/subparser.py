import argparse

from . import utils


def run(parser: utils.Parser, args):
    subcommand, data = args
    if subcommand == "init":
        metaname, kwargs = data
        parser.add_subparser(metaname, **kwargs)
    elif subcommand == "create":
        name, metaname, kwargs = data
        parser.add_parser(metaname, name, **kwargs)


def do_init(args):
    data = utils.arglist_to_kwargs(args.rest)
    print(
        utils.append_to_state(args.state, "subparser", ("init", (args.metaname, data)))
    )


def do_create(args):
    data = utils.arglist_to_kwargs(args.rest)
    print(
        utils.append_to_state(
            args.state, "subparser", ("create", (args.name, args.metaname, data))
        )
    )


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("state")
    parser.add_argument("--metaname", required=False, default=None)

    subparsers = parser.add_subparsers(required=True)

    init = subparsers.add_parser("init")
    init.set_defaults(func=do_init)

    create = subparsers.add_parser("create")
    create.add_argument("name")
    create.set_defaults(func=do_create)

    parser.add_argument("rest", nargs=argparse.REMAINDER)

    args = parser.parse_args()
    args.func(args)

import argparse

from . import utils


def pack_args(args: argparse.Namespace):
    # add an argument to obj by assembling the method to call
    aliases = []
    while len(args.rest) and not args.rest[0] == "--":
        aliases.append(args.rest[0])
        args.rest.pop(0)
    meth_args = aliases

    if len(args.rest):
        args.rest.pop(0)

    meth_kwargs = utils.arglist_to_kwargs(args.rest)
    return utils.append_to_state(
        args.object,
        "add_arg",
        (args.subparser, args.parser_arg, meth_args, meth_kwargs),
    )


def run(parser: utils.Parser, packed):
    subparser, parser_arg, meth_args, meth_kwargs = packed
    p = parser.get_parser(parser_arg, subparser)
    p.add_argument(*meth_args, **meth_kwargs)


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("object")
    parser.add_argument("--subparser", default=None)
    parser.add_argument("--parser-arg", default=None)
    parser.add_argument("rest", nargs=argparse.REMAINDER)
    args = parser.parse_args()

    print(pack_args(args))

import argparse

from . import utils


def pack_args(args: argparse.Namespace):
    meth_kwargs = utils.arglist_to_kwargs(args.rest)
    return utils.append_to_state(
        args.object, "set_defaults", (args.subparser, args.parser_arg, meth_kwargs)
    )


def run(parser: utils.Parser, packed):
    subparser, parser_arg, meth_kwargs = packed
    p = parser.get_parser(parser_arg, subparser)
    p.set_defaults(**meth_kwargs)


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("object", help="parser state")
    parser.add_argument(
        "--subparser",
        help="Name of subparser command (argument to create)",
        default=None,
    )
    parser.add_argument(
        "--parser-arg",
        help="Name of subparser argument (argument to init)",
        default=None,
    )
    parser.add_argument("rest", nargs=argparse.REMAINDER)
    args = parser.parse_args()

    print(pack_args(args))

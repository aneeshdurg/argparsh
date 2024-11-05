import argparse
import sys

from . import utils
from . import add_arg
from . import new
from . import subparser
from . import set_defaults

name_to_module = {
    "add_arg": add_arg,
    "new": new,
    "subparser": subparser,
    "set_defaults": set_defaults,
}

if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("object")
    parser.add_argument("rest", nargs=argparse.REMAINDER)
    args = parser.parse_args()

    actions = utils.parse_state(args.object)

    new_parser = utils.Parser()
    for name, data in actions:
        name_to_module[name].run(new_parser, data)

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

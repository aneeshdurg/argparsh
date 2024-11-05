import argparse

from . import utils


def run(parser: utils.Parser, args):
    parser.initialize(**args)


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("name")
    parser.add_argument("-d", "--description", action="store", default="")
    parser.add_argument("-e", "--epilog", action="store", default="")
    args = parser.parse_args()

    kwargs = {
        "prog": args.name,
        "description": args.description,
        "epilog": args.epilog,
    }
    print(utils.append_to_state("", "new", kwargs))

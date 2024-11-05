import argparse
import urllib.parse
import sys

parser = argparse.ArgumentParser()
parser.add_argument('object')
parser.add_argument('method', choices=["init", "add_arg", "parse"])
parser.add_argument('rest', nargs=argparse.REMAINDER)
args = parser.parse_args()

obj = urllib.parse.unquote_plus(args.object)
if args.method == "parse":
    obj += "parsed_args = parser.parse_args(args)\n"
    locals_ = {"args": args.rest}

    tmp_stdout = sys.stdout
    sys.stdout = sys.stderr
    exit_ = None
    try:
        exec(obj, {"argparse": argparse}, locals_)
    except SystemExit as e:
        exit_ = e
    finally:
        sys.stdout = tmp_stdout

    if exit_:
        print(f"exit {exit_}")
    else:
        args_ = locals_["parsed_args"]
        for k, v in args_._get_kwargs():
            print(f"{k.upper()}={repr(v)}")
    exit(0)

if args.method == "init":
    assert obj == ""
    obj += "parser = argparse.ArgumentParser()\n"
else:
    # add an argument to obj by assembling the method to call
    aliases = []
    while len(args.rest) and not args.rest[0] == '--':
        aliases.append(args.rest[0])
        args.rest.pop(0)
    call = "parser.add_argument(" + repr(aliases[0])
    aliases.pop(0)
    if len(aliases):
        call += ", "
        call += ", ".join(repr('--' + a) for a in aliases)

    if len(args.rest):
        args.rest.pop(0)
    print(args.rest, file=sys.stderr)
    call += ")\n"
    obj += call
print(urllib.parse.quote_plus(obj))

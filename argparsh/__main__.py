import argparse
import urllib.parse
import sys
import pickle

parser = argparse.ArgumentParser()
parser.add_argument('object')
parser.add_argument('method', choices=["add_arg", "parse"])
parser.add_argument('rest', nargs=argparse.REMAINDER)
args = parser.parse_args()

obj = args.object
if args.method == "parse":
    new_parser = argparse.ArgumentParser()

    argument_calls = obj.split('+')[1:]
    for call in argument_calls:
        meth_args, meth_kwargs = pickle.loads(urllib.parse.unquote_to_bytes(call))
        new_parser.add_argument(*meth_args, **meth_kwargs)

    output = sys.stdout
    sys.stdout = sys.stderr
    exit_ = None
    try:
        parsed_args = new_parser.parse_args(args.rest)
        for k, v in parsed_args._get_kwargs():
            print(f"{k.upper()}={repr(v)}", file=output)
    except SystemExit as e:
        print(f"exit {e}", file=output)
        exit_ = e
    exit(0)


# add an argument to obj by assembling the method to call
aliases = []
while len(args.rest) and not args.rest[0] == '--':
    aliases.append(args.rest[0])
    args.rest.pop(0)
meth_args = aliases

if len(args.rest):
    args.rest.pop(0)

meth_kwargs = {}
for i in range(0, len(args.rest), 2):
    assert args.rest[i].startswith('--')
    key = args.rest[i][2:]
    value = args.rest[i + 1]
    meth_kwargs[key] = eval(value, {}, {})
bytes_ = pickle.dumps((meth_args, meth_kwargs))
obj += "+" + urllib.parse.quote_from_bytes(bytes_)
print(obj)

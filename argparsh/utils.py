import pickle
import urllib.parse
import argparse
from dataclasses import dataclass, field
import uuid


def output(method: str, data):
    bytes_ = pickle.dumps((method, data))
    return "+" + urllib.parse.quote_from_bytes(bytes_)


def parse_state(state: str):
    return [
        pickle.loads(urllib.parse.unquote_to_bytes(x)) for x in state.split("+")[1:]
    ]


def arglist_to_kwargs(arglist):
    kwargs = {}
    for i in range(0, len(arglist), 2):
        assert arglist[i].startswith("--")
        key = arglist[i][2:]
        value = arglist[i + 1]
        if key in ["type", "choices"]:
            kwargs[key] = eval(value, {}, {})
        elif key == "nargs":
            try:
                kwargs[key] = int(value)
            except ValueError:
                kwargs[key] = value
        elif key in ["deprecated", "required"]:
            kwargs[key] = bool(value)
        else:
            kwargs[key] = value
    return kwargs


@dataclass
class Parser:
    _parser: argparse.ArgumentParser | None = None
    _subparsers: dict[str, argparse._SubParsersAction] = field(default_factory=dict)

    active_parser = None

    def initialize(self, *args, **kwargs):
        assert self._parser is None
        self._parser = argparse.ArgumentParser(
            *args, **kwargs, formatter_class=argparse.RawTextHelpFormatter
        )

    @property
    def parser(self) -> argparse.ArgumentParser:
        if self._parser is None:
            self._parser = argparse.ArgumentParser()
        if self.active_parser is not None:
            return self.active_parser
        return self._parser

    def add_subparser(self, **kwargs):
        metaname = str(uuid.uuid4())
        p = self.parser
        self._subparsers[metaname] = p.add_subparsers(**kwargs)
        return metaname

    def add_parser(self, metaname, name, **kwargs):
        new_parser = self._subparsers[metaname].add_parser(name, **kwargs)
        return new_parser

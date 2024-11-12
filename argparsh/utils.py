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
    _parsers: dict[str, dict[str, argparse.ArgumentParser]] = field(
        default_factory=dict
    )

    def initialize(self, *args, **kwargs):
        self._parser = argparse.ArgumentParser(
            *args, **kwargs, formatter_class=argparse.RawTextHelpFormatter
        )

    @property
    def parser(self) -> argparse.ArgumentParser:
        if self._parser is None:
            self._parser = argparse.ArgumentParser()
        return self._parser

    def add_subparser(self, metaname, **kwargs):
        if metaname is None:
            metaname = str(uuid.uuid4())
        self._subparsers[metaname] = self.parser.add_subparsers(**kwargs)

    def add_parser(self, metaname, name, **kwargs):
        if metaname is None:
            metaname = list(self._subparsers.keys())[-1]
        if metaname not in self._parsers:
            self._parsers[metaname] = {}
        self._parsers[metaname][name] = self._subparsers[metaname].add_parser(
            name, **kwargs
        )

    def get_parser(self, metaname, name):
        if metaname is None and name is None:
            return self.parser

        if metaname is None:
            metaname = list(self._subparsers.keys())[-1]
        assert (
            metaname in self._subparsers
        ), f"Could not find parser with name {metaname}"
        assert (
            name in self._parsers[metaname]
        ), f"Could not find subparser with name {name}"
        return self._parsers[metaname][name]

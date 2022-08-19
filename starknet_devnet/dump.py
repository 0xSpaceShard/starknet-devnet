"""Dumping utilities."""

import multiprocessing

import cloudpickle as pickle

from .devnet_config import DumpOn

# Instead of "fork", the default on MacOS since Python3.8 has been "spawn", which causes pickling to fail
multiprocessing.set_start_method("fork")

class Dumper:
    """Class for dumping objects."""

    def __init__(self, dumpable):
        """Specify the `dumpable` object to be dumped."""

        self.dumpable = dumpable

        self.dump_path: str = None
        """Where to dump."""

        self.dump_on: DumpOn = None
        """When to dump."""

    def __write_file(self, path):
        """Writes the dump to disk."""
        with open(path, "wb") as file:
            pickle.dump(self.dumpable, file)

    def dump(self, path: str=None):
        """Dump to `path`."""
        path = path or self.dump_path
        assert path, "No dump_path defined"

        print("Dumping Devnet to:", path)
        self.__write_file(path)

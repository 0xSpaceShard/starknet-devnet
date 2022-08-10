"""
A server exposing Starknet functionalities as API endpoints.
"""

from pickle import UnpicklingError
import sys
import asyncio

from flask import Flask, jsonify
from flask_cors import CORS
from gunicorn.app.base import BaseApplication
from starkware.starkware_utils.error_handling import StarkException

from .blueprints.base import base
from .blueprints.gateway import gateway
from .blueprints.feeder_gateway import feeder_gateway
from .blueprints.postman import postman
from .blueprints.rpc import rpc
from .util import check_valid_dump_path
from .state import state
from .devnet_config import devnet_config, DumpOn

app = Flask(__name__)
CORS(app)

@app.before_first_request
async def initialize_starknet():
    """Initialize Starknet to assert it's defined before its first use."""
    await state.starknet_wrapper.initialize()

app.register_blueprint(base)
app.register_blueprint(gateway)
app.register_blueprint(feeder_gateway)
app.register_blueprint(postman)
app.register_blueprint(rpc)

def set_dump_options(args):
    """Assign dumping options from args to state."""
    if args.dump_path:
        try:
            check_valid_dump_path(args.dump_path)
        except ValueError as error:
            sys.exit(str(error))

    state.dumper.dump_path = args.dump_path
    state.dumper.dump_on = args.dump_on

def load_dumped(args):
    """Load a previously dumped state if specified."""
    if args.load_path:
        try:
            state.load(args.load_path)
        except (FileNotFoundError, UnpicklingError):
            sys.exit(f"Error: Cannot load from {args.load_path}. Make sure the file exists and contains a Devnet dump.")

# We don't need init method here.
# pylint: disable=W0223
class Devnet(BaseApplication):
    """Our Gunicorn application."""

    def __init__(self, application, args):
        self.args = args
        self.application = application
        super().__init__()

    def load_config(self):
        self.cfg.set("bind", f"{self.args.host}:{self.args.port}")
        self.cfg.set("workers", 1)
        self.cfg.set("logconfig_dict", {
            "loggers": {
                "gunicorn.error": {
                    # Disable info messages like "Starting gunicorn"
                    "level": "WARNING",
                    "handlers": ["error_console"],
                    "propagate": False,
                    "qualname": "gunicorn.error"
                },

                "gunicorn.access": {
                    "level": "INFO",
                    # Log access to stderr to maintain backward compatibility
                    "handlers": ["error_console"],
                    "propagate": False,
                    "qualname": "gunicorn.access"
                }
            },
        })

    def load(self):
        return self.application


def main():
    """Runs the server."""

    # Uncomment this once fork support is added
    # origin = Origin(args.fork) if args.fork else NullOrigin()
    # starknet_wrapper.origin = origin

    args = devnet_config.args
    load_dumped(args)
    set_dump_options(args)

    asyncio.run(state.starknet_wrapper.initialize())

    try:
        print(f" * Listening on http://{args.host}:{args.port}/ (Press CTRL+C to quit)")
        Devnet(app, args).run()
    except KeyboardInterrupt:
        pass
    finally:
        if args.dump_on == DumpOn.EXIT:
            state.dumper.dump()
            sys.exit(0)

@app.errorhandler(StarkException)
def handle(error: StarkException):
    """Handles the error and responds in JSON. """
    return {"message": error.message, "status_code": error.status_code}, error.status_code

@app.route("/api", methods = ["GET"])
def api():
    """Return available endpoints."""
    routes = {}
    for url in app.url_map.iter_rules():
        if url.endpoint != "static":
            routes[url.rule] = {
                "functionName": url.endpoint,
                "methods": list(url.methods),
                "doc": app.view_functions[url.endpoint].__doc__.strip()
            }
    return jsonify(routes)

if __name__ == "__main__":
    main()

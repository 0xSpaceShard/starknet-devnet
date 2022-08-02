"""
A server exposing Starknet functionalities as API endpoints.
"""

from pickle import UnpicklingError
import sys

from flask import Flask, jsonify
from flask_cors import CORS
import meinheld
from starkware.starkware_utils.error_handling import StarkException

from .blueprints.base import base
from .blueprints.gateway import gateway
from .blueprints.feeder_gateway import feeder_gateway
from .blueprints.postman import postman
from .blueprints.rpc import rpc
from .util import DumpOn, check_valid_dump_path, parse_args
from .starknet_wrapper import DevnetConfig
from .state import state

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

def generate_accounts(args):
    """Generate accounts """
    if args.accounts:
        state.starknet_wrapper.accounts.generate(
            n_accounts=args.accounts,
            initial_balance=args.initial_balance,
            seed=args.seed
        )
        state.starknet_wrapper.accounts.print()

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

def enable_lite_mode(args):
    """Enable lite mode if specified."""
    if args.lite_mode:
        config = DevnetConfig(
            lite_mode_block_hash=True,
            lite_mode_deploy_hash=True
        )
    else:
        config = DevnetConfig(
            lite_mode_block_hash=args.lite_mode_block_hash,
            lite_mode_deploy_hash=args.lite_mode_deploy_hash
        )

    state.starknet_wrapper.set_config(config)

def set_start_time(args):
    """Assign start time if specified."""
    if args.start_time is not None:
        state.starknet_wrapper.set_block_time(args.start_time)

def set_gas_price(args):
    """Assign gas_price"""
    state.starknet_wrapper.set_gas_price(args.gas_price)

def main():
    """Runs the server."""

    args = parse_args()

    # Uncomment this once fork support is added
    # origin = Origin(args.fork) if args.fork else NullOrigin()
    # starknet_wrapper.origin = origin

    load_dumped(args)
    set_dump_options(args)
    generate_accounts(args)
    enable_lite_mode(args)
    set_start_time(args)
    set_gas_price(args)

    try:
        meinheld.listen((args.host, args.port))
        print(f" * Listening on http://{args.host}:{args.port}/ (Press CTRL+C to quit)")
        meinheld.run(app)
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

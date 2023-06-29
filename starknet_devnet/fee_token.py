"""
Fee token and its predefined constants.
"""
import pprint
import sys

from starkware.solidity.utils import load_nearby_contract
from starkware.starknet.business_logic.state.storage_domain import StorageDomain
from starkware.starknet.business_logic.transaction.objects import InternalInvokeFunction
from starkware.starknet.compiler.compile import get_selector_from_name
from starkware.starknet.services.api.contract_class.contract_class import (
    CompiledClassBase,
    DeprecatedCompiledClass,
)
from starkware.starknet.services.api.gateway.transaction import InvokeFunction
from starkware.starknet.testing.starknet import Starknet

from starknet_devnet.account_util import get_execute_args
from starknet_devnet.chargeable_account import ChargeableAccount
from starknet_devnet.constants import SUPPORTED_TX_VERSION
from starknet_devnet.predeployed_contract_wrapper import PredeployedContractWrapper
from starknet_devnet.util import Uint256, logger, str_to_felt


class FeeToken(PredeployedContractWrapper):
    """Wrapper of token for charging fees."""

    CONTRACT_CLASS: CompiledClassBase = None  # loaded lazily

    # Precalculated
    # HASH = compute_deprecated_class_hash(contract_class=FeeToken.get_contract_class())
    HASH = 0x6A22BF63C7BC07EFFA39A25DFBD21523D211DB0100A0AFD054D172B81840EAF

    # Taken from
    # https://github.com/starknet-community-libs/starknet-addresses/blob/df19b17d2c83f11c30e65e2373e8a0c65446f17c/bridged_tokens/goerli.json
    ADDRESS = 0x49D36570D4E46F48E99674BD3FCC84644DDD6B96F7C741B1562B82F9E004DC7
    SYMBOL = "ETH"
    NAME = "ether"

    def __init__(self, starknet_wrapper):
        self.starknet_wrapper = starknet_wrapper
        self.address = self.ADDRESS
        self.class_hash = self.HASH

    @classmethod
    def get_contract_class(cls) -> CompiledClassBase:
        """Returns contract class via lazy loading."""
        if not cls.CONTRACT_CLASS:
            cls.CONTRACT_CLASS = DeprecatedCompiledClass.load(
                load_nearby_contract("ERC20_Mintable_OZ_0.2.0")
            )
        return cls.CONTRACT_CLASS

    @property
    def contract_class(self) -> DeprecatedCompiledClass:
        """Same as `get_contract_class`, used by `PredeployedContractWrapper` parent"""
        return self.get_contract_class()

    async def _mimic_constructor(self):
        starknet: Starknet = self.starknet_wrapper.starknet
        await starknet.state.state.set_storage_at(
            storage_domain=StorageDomain.ON_CHAIN,
            contract_address=FeeToken.ADDRESS,
            key=get_selector_from_name("ERC20_name"),
            value=str_to_felt(FeeToken.NAME),
        )
        await starknet.state.state.set_storage_at(
            storage_domain=StorageDomain.ON_CHAIN,
            contract_address=FeeToken.ADDRESS,
            key=get_selector_from_name("ERC20_symbol"),
            value=str_to_felt(FeeToken.SYMBOL),
        )
        await starknet.state.state.set_storage_at(
            storage_domain=StorageDomain.ON_CHAIN,
            contract_address=FeeToken.ADDRESS,
            key=get_selector_from_name("ERC20_decimals"),
            value=18,
        )
        await starknet.state.state.set_storage_at(
            storage_domain=StorageDomain.ON_CHAIN,
            contract_address=FeeToken.ADDRESS,
            key=get_selector_from_name("Ownable_owner"),
            value=ChargeableAccount.ADDRESS,
        )

    async def get_balance(self, address: int) -> int:
        """Return the balance of the contract under `address`."""
        response = await self.contract.balanceOf(address).call()

        balance = Uint256(
            low=response.result.balance.low, high=response.result.balance.high
        ).to_felt()
        return balance

    async def get_mint_transaction(self, fundable_address: int, amount: Uint256):
        """Construct a transaction object representing minting request"""

        starknet: Starknet = self.starknet_wrapper.starknet
        calldata = [
            str(fundable_address),
            str(amount.low),
            str(amount.high),
        ]

        version = SUPPORTED_TX_VERSION
        max_fee = int(1e18)  # big enough

        # we need a funded account for this since the tx has to be signed and a fee will be charged
        # a user-intedded predeployed account cannot be used for this
        nonce = await starknet.state.state.get_nonce_at(
            StorageDomain.ON_CHAIN, ChargeableAccount.ADDRESS
        )
        chargeable_address = hex(ChargeableAccount.ADDRESS)
        signature, execute_calldata = get_execute_args(
            calls=[(hex(FeeToken.ADDRESS), "mint", calldata)],
            account_address=chargeable_address,
            private_key=ChargeableAccount.PRIVATE_KEY,
            nonce=nonce,
            version=version,
            max_fee=max_fee,
            chain_id=starknet.state.general_config.chain_id,
        )

        transaction_data = {
            "calldata": [str(v) for v in execute_calldata],
            "contract_address": chargeable_address,
            "nonce": hex(nonce),
            "max_fee": hex(max_fee),
            "signature": signature,
            "version": hex(version),
        }
        return InvokeFunction.load(transaction_data)

    async def mint(self, to_address: int, amount: int, lite: bool):
        """
        Mint `amount` tokens at address `to_address`.
        Returns the `tx_hash` (as hex str) if not `lite`; else returns `None`
        """
        amount_uint256 = Uint256.from_felt(amount)

        tx_hash = None
        transaction = await self.get_mint_transaction(to_address, amount_uint256)
        logger.info(transaction)
        starknet: Starknet = self.starknet_wrapper.starknet
        if lite:
            internal_tx = InternalInvokeFunction.from_external(
                transaction, starknet.state.general_config
            )
            execution_info = await starknet.state.execute_tx(internal_tx)
            logger.info(
                "transaction execution info: %s", pprint.pformat(execution_info.dump())
            )
        else:
            # execution info logs inside starknet_wrapper.invoke call
            _, tx_hash_int = await self.starknet_wrapper.invoke(transaction)
            tx_hash = hex(tx_hash_int)

        return tx_hash

    def print(self):
        print("")
        print("Predeployed FeeToken")
        print(f"Address: {hex(self.address)}")
        print(f"Class Hash: {hex(self.class_hash)}")
        print(f"Symbol: {self.SYMBOL}\n")
        sys.stdout.flush()

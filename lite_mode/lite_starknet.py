"""
This module introduces `LiteStarknet`, optimized lite-mode version of Starknet.
"""
from typing import List, Union, Optional

from starkware.python.utils import as_non_optional
from starkware.starknet.testing.starknet import Starknet
from starkware.starknet.testing.objects import StarknetCallInfo
from starkware.starknet.services.api.contract_class import ContractClass
from starkware.starknet.testing.contract import StarknetContract
from starkware.starknet.testing.contract_utils import (
    get_abi,
    get_contract_class,
)

from lite_mode.lite_starknet_state import LiteStarknetState

CastableToAddressSalt = Union[str, int]

# pylint: disable=too-many-arguments, arguments-differ)
class LiteStarknet(Starknet):
    """
    The lite version of Starknet which avoid transaction hash calculation in deploy.
    """

    async def deploy(
        self,
        starknet: Starknet,
        tx_number: int,
        source: Optional[str] = None,
        contract_class: Optional[ContractClass] = None,
        contract_address_salt: Optional[CastableToAddressSalt] = None,
        cairo_path: Optional[List[str]] = None,
        constructor_calldata: Optional[List[int]] = None,
        disable_hint_validation: bool = False,
    ) -> StarknetContract:
        contract_class = get_contract_class(
            source=source,
            contract_class=contract_class,
            cairo_path=cairo_path,
            disable_hint_validation=disable_hint_validation,
        )

        address, execution_info = await LiteStarknetState.deploy(
            self,
            contract_class=contract_class,
            contract_address_salt=contract_address_salt,
            constructor_calldata=[]
            if constructor_calldata is None
            else constructor_calldata,
            starknet=starknet,
            tx_number=tx_number,
        )

        deploy_call_info = StarknetCallInfo.from_internal(
            call_info=as_non_optional(execution_info.call_info),
            result=(),
            main_call_events=[],
        )

        return StarknetContract(
            state=starknet.state,
            abi=get_abi(contract_class=contract_class),
            contract_address=address,
            deploy_call_info=deploy_call_info,
        )

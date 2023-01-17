"""
Account that is charged with a fee when nobody else can be charged.
"""

from starknet_devnet.account import Account


class ChargeableAccount(Account):
    """
    A well-funded account that can be charged with a fee when no other account can.
    E.g. for signing mint txs. Can also be useful in tests.
    """

    PRIVATE_KEY = 0x5FB2959E3011A873A7160F5BB32B0ECE
    PUBLIC_KEY = 0x4C37AB4F0994879337BFD4EAD0800776DB57DA382B8ED8EFAA478C5D3B942A4
    ADDRESS = 0x1CAF2DF5ED5DDE1AE3FAEF4ACD72522AC3CB16E23F6DC4C7F9FAED67124C511

    def __init__(self, starknet_wrapper):
        super().__init__(
            starknet_wrapper,
            private_key=ChargeableAccount.PRIVATE_KEY,
            public_key=ChargeableAccount.PUBLIC_KEY,
            initial_balance=2**251,  # loads of cash
            account_class_wrapper=starknet_wrapper.config.account_class,
        )

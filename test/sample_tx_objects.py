"""
Contains sample tx objects created with:
    initial_balance = "10"
    deploy_info = deploy(contract=CONTRACT_PATH, inputs=[initial_balance], salt="0x42")
    contract_address = deploy_info["address"]

    get_estimated_fee(
        calls=[(contract_address, "increase_balance", [0, 0])],
        account_address=PREDEPLOYED_ACCOUNT_ADDRESS,
        private_key=PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
    )
    get_estimated_fee(
        calls=[(contract_address, "increase_balance", [10, 20])],
        account_address=PREDEPLOYED_ACCOUNT_ADDRESS,
        private_key=PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
        nonce=1
    )
"""

TX_DICT1 = {
    "contract_address": "0x347be35996a21f6bf0623e75dbce52baba918ad5ae8d83b6f416045ab22961a",
    "max_fee": "0x0",
    "calldata": [
        "1",
        "1899433162421946587614251337844985054247944657949259912835517185361206070561",
        "1530486729947006463063166157847785599120665941190480211966374137237989315360",
        "0",
        "2",
        "2",
        "0",
        "0",
    ],
    "version": "0x100000000000000000000000000000001",
    "nonce": "0x0",
    "signature": [
        "3174996110485379956944934596703052164206842524356023427475484042061844533395",
        "17577146695708340226702412773352693706935620568901342811436914942566987841",
    ],
    "type": "INVOKE_FUNCTION",
}

TX_DICT2 = {
    "max_fee": "0x0",
    "signature": [
        "2392843069849124933012063680022083930445422419478594435201358295535133395926",
        "1905143518502940252114978183829346881587323602689293951107361008135757778913",
    ],
    "calldata": [
        "1",
        "1899433162421946587614251337844985054247944657949259912835517185361206070561",
        "1530486729947006463063166157847785599120665941190480211966374137237989315360",
        "0",
        "2",
        "2",
        "10",
        "20",
    ],
    "version": "0x100000000000000000000000000000001",
    "contract_address": "0x347be35996a21f6bf0623e75dbce52baba918ad5ae8d83b6f416045ab22961a",
    "nonce": "0x1",
    "type": "INVOKE_FUNCTION",
}

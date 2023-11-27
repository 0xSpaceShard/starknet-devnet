// SPDX-License-Identifier: MIT
pragma solidity ^0.6.12;

import "forge-std/Script.sol";
import "forge-std/console.sol";

import "src/MockStarknetMessaging.sol";
import "src/L1L2.sol";

contract Deploy is Script {

    function setUp() public {
    }

    function run() public {
        // .env file is automatically sourced by forge.
        uint256 deployerPrivateKey = vm.envUint("ACCOUNT_PRIVATE_KEY");
        MockStarknetMessaging snMessaging = MockStarknetMessaging(vm.envAddress("STARKNET_MESSAGING_ADDRESS"));

        vm.startBroadcast(deployerPrivateKey);

        address addr = address(new L1L2Example(snMessaging));

        console.log(addr);

        vm.stopBroadcast();
    }
}

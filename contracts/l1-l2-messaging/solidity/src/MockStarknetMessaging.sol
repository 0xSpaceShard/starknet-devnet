// SPDX-License-Identifier: Apache-2.0.
pragma solidity ^0.6.12;

import "./StarknetMessaging.sol";

contract MockStarknetMessaging is StarknetMessaging {
    constructor(uint256 MessageCancellationDelay) public {
        messageCancellationDelay(MessageCancellationDelay);
    }

    /**
      Mocks a message from L2 to L1.
    */
    function mockSendMessageFromL2(
        uint256 fromAddress,
        uint256 toAddress,
        uint256[] calldata payload
    ) external {
        bytes32 msgHash = keccak256(
            abi.encodePacked(fromAddress, toAddress, payload.length, payload)
        );
        l2ToL1Messages()[msgHash] += 1;

        // Devnet-specific modification to trigger the event
        emit LogMessageToL1(fromAddress, address(toAddress), payload);
    }

    /**
      Mocks consumption of a message from L2 to L1.
    */
    function mockConsumeMessageFromL2(
        uint256 fromAddress,
        uint256 toAddress,
        uint256[] calldata payload
    ) external {
        bytes32 msgHash = keccak256(
            abi.encodePacked(fromAddress, toAddress, payload.length, payload)
        );

        require(l2ToL1Messages()[msgHash] > 0, "INVALID_MESSAGE_TO_CONSUME");
        emit ConsumedMessageToL1(fromAddress, msg.sender, payload);
        l2ToL1Messages()[msgHash] -= 1;
    }
}

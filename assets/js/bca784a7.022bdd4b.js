"use strict";(self.webpackChunkwebsite=self.webpackChunkwebsite||[]).push([[8626],{5787:(e,n,t)=>{t.r(n),t.d(n,{assets:()=>d,contentTitle:()=>o,default:()=>h,frontMatter:()=>a,metadata:()=>i,toc:()=>c});var s=t(4848),r=t(8453);const a={},o="L1-L2 interaction via Postman",i={id:"postman",title:"L1-L2 interaction via Postman",description:"Postman is a Starknet utility that allows testing L1-L2 interaction. It is unrelated to the Postman API platform. To use it, ensure you have:",source:"@site/docs/postman.md",sourceDirName:".",slug:"/postman",permalink:"/starknet-devnet/docs/next/postman",draft:!1,unlisted:!1,editUrl:"https://github.com/0xSpaceShard/starknet-devnet/blob/master/website/docs/postman.md",tags:[],version:"current",frontMatter:{},sidebar:"docSidebar",previous:{title:"Lite mode",permalink:"/starknet-devnet/docs/next/lite"},next:{title:"Predeployed contracts",permalink:"/starknet-devnet/docs/next/predeployed"}},d={},c=[{value:"Load",id:"load",level:2},{value:"L1 network",id:"l1-network",level:3},{value:"Flush",id:"flush",level:2},{value:"Disclaimer",id:"disclaimer",level:2},{value:"Mock transactions",id:"mock-transactions",level:2},{value:"L1-&gt;L2",id:"l1-l2",level:3},{value:"L2-&gt;L1",id:"l2-l1",level:3}];function l(e){const n={a:"a",admonition:"admonition",code:"code",h1:"h1",h2:"h2",h3:"h3",li:"li",p:"p",pre:"pre",strong:"strong",ul:"ul",...(0,r.R)(),...e.components};return(0,s.jsxs)(s.Fragment,{children:[(0,s.jsx)(n.h1,{id:"l1-l2-interaction-via-postman",children:"L1-L2 interaction via Postman"}),"\n",(0,s.jsxs)(n.p,{children:["Postman is a Starknet utility that allows testing L1-L2 interaction. It is ",(0,s.jsx)(n.strong,{children:"unrelated"})," to the ",(0,s.jsx)(n.a,{href:"https://www.postman.com/",children:"Postman API platform"}),". To use it, ensure you have:"]}),"\n",(0,s.jsxs)(n.ul,{children:["\n",(0,s.jsxs)(n.li,{children:["an L1 node (possibilities listed ",(0,s.jsx)(n.a,{href:"#l1-network",children:"below"}),")"]}),"\n",(0,s.jsx)(n.li,{children:"a Devnet instance (acting as L2 node)"}),"\n",(0,s.jsxs)(n.li,{children:["a ",(0,s.jsx)(n.a,{href:"#load",children:"loaded"})," messaging contract","\n",(0,s.jsxs)(n.ul,{children:["\n",(0,s.jsx)(n.li,{children:"this is an L1 contract for exchanging messages between L1 and L2"}),"\n",(0,s.jsx)(n.li,{children:"you can deploy a new instance or specify the address of an old one"}),"\n"]}),"\n"]}),"\n",(0,s.jsx)(n.li,{children:"an L1 contract that can interact with the messaging contract"}),"\n",(0,s.jsx)(n.li,{children:"an L2 contract that can interact with the messaging contract"}),"\n"]}),"\n",(0,s.jsxs)(n.p,{children:["There are two internal message queues: one for L1->L2 messages, another for L2->L1 messages. When there are messages in a queue, you will need to ",(0,s.jsx)(n.a,{href:"#flush",children:"flush"})," to transmit the messages to their destinations."]}),"\n",(0,s.jsxs)(n.p,{children:["You can use ",(0,s.jsx)(n.a,{href:"https://github.com/0xSpaceShard/starknet-devnet-js",children:(0,s.jsx)(n.strong,{children:(0,s.jsx)(n.code,{children:"starknet-devnet-js"})})})," to assist you in the above listed actions. ",(0,s.jsx)(n.a,{href:"https://github.com/0xSpaceShard/starknet-devnet-js/blob/master/test/l1-l2-postman.test.ts",children:(0,s.jsx)(n.strong,{children:"This example"})}),", especially the ",(0,s.jsx)(n.code,{children:'it("should exchange messages between L1 and L2")'})," test case should be of most help. The required contracts are configured and deployed in the ",(0,s.jsx)(n.code,{children:"before"})," block. Alternatively, you can directly send requests to the endpoints specified below."]}),"\n",(0,s.jsx)(n.h2,{id:"load",children:"Load"}),"\n",(0,s.jsx)(n.pre,{children:(0,s.jsx)(n.code,{children:"POST /postman/load_l1_messaging_contract\n"})}),"\n",(0,s.jsx)(n.pre,{children:(0,s.jsx)(n.code,{className:"language-json",children:'{\n  "network_url": "http://localhost:8545",\n  "address": "0x123...def" // optional\n}\n'})}),"\n",(0,s.jsx)(n.pre,{children:(0,s.jsx)(n.code,{children:'JSON-RPC\n{\n    "jsonrpc": "2.0",\n    "id": "1",\n    "method": "devnet_postmanLoad",\n    "params": {\n      "network_url": "http://localhost:8545",\n      "address": "0x123...def"\n    }\n}\n'})}),"\n",(0,s.jsxs)(n.p,{children:["Loads a ",(0,s.jsx)(n.code,{children:"MockStarknetMessaging"})," contract. The ",(0,s.jsx)(n.code,{children:"address"})," parameter is optional; if provided, the ",(0,s.jsx)(n.code,{children:"MockStarknetMessaging"})," contract will be fetched from that address, otherwise a new one will be deployed."]}),"\n",(0,s.jsxs)(n.admonition,{title:"L1-L2 with dockerized Devnet",type:"note",children:[(0,s.jsxs)(n.p,{children:["L1-L2 communication requires extra attention if Devnet is ",(0,s.jsx)(n.a,{href:"/starknet-devnet/docs/next/running/docker",children:"run in a Docker container"}),". The ",(0,s.jsx)(n.code,{children:"network_url"})," argument must be on the same network as Devnet. E.g. if your L1 instance is run locally (i.e. using the host machine's network), then:"]}),(0,s.jsxs)(n.ul,{children:["\n",(0,s.jsxs)(n.li,{children:["on Linux, it is enough to run the Devnet Docker container with ",(0,s.jsx)(n.code,{children:"--network host"})]}),"\n",(0,s.jsxs)(n.li,{children:["on Mac and Windows, replace any ",(0,s.jsx)(n.code,{children:"http://localhost"})," or ",(0,s.jsx)(n.code,{children:"http://127.0.0.1"})," occurrence in the value of ",(0,s.jsx)(n.code,{children:"network_url"})," with ",(0,s.jsx)(n.code,{children:"http://host.docker.internal"}),"."]}),"\n"]})]}),"\n",(0,s.jsx)(n.h3,{id:"l1-network",children:"L1 network"}),"\n",(0,s.jsxs)(n.p,{children:["The ",(0,s.jsx)(n.code,{children:"network_url"})," parameter refers to the URL of the JSON-RPC API endpoint of the L1 node you've run locally, or which is publicly accessible. Possibilities include, but are not limited to:"]}),"\n",(0,s.jsxs)(n.ul,{children:["\n",(0,s.jsx)(n.li,{children:(0,s.jsx)(n.a,{href:"https://github.com/foundry-rs/foundry/tree/master#anvil",children:(0,s.jsx)(n.strong,{children:"Anvil"})})}),"\n",(0,s.jsx)(n.li,{children:(0,s.jsx)(n.a,{href:"https://sepolia.etherscan.io/",children:(0,s.jsx)(n.strong,{children:"Sepolia testnet"})})}),"\n",(0,s.jsx)(n.li,{children:(0,s.jsx)(n.a,{href:"https://www.npmjs.com/package/ganache",children:(0,s.jsx)(n.strong,{children:"Ganache"})})}),"\n",(0,s.jsx)(n.li,{children:(0,s.jsx)(n.a,{href:"https://github.com/ethereum/go-ethereum#docker-quick-start",children:(0,s.jsx)(n.strong,{children:"Geth"})})}),"\n",(0,s.jsx)(n.li,{children:(0,s.jsx)(n.a,{href:"https://hardhat.org/hardhat-network/#running-stand-alone-in-order-to-support-wallets-and-other-software",children:(0,s.jsx)(n.strong,{children:"Hardhat node"})})}),"\n"]}),"\n",(0,s.jsx)(n.admonition,{title:"Dumping and Loading",type:"info",children:(0,s.jsxs)(n.p,{children:["Loading a messaging contract is a dumpable event, meaning that, if you've enabled dumping, a messaging-contract-loading event will be dumped. Keep in mind that, if you rely on Devnet deploying a new contract, i.e. if you don't specify a contract address of an already deployed messaging contract, a new contract will be deployed at a new address on each loading of the dump. Read more about dumping ",(0,s.jsx)(n.a,{href:"./dump-load-restart#dumping",children:"here"}),"."]})}),"\n",(0,s.jsx)(n.h2,{id:"flush",children:"Flush"}),"\n",(0,s.jsx)(n.pre,{children:(0,s.jsx)(n.code,{children:"POST /postman/flush\n"})}),"\n",(0,s.jsx)(n.pre,{children:(0,s.jsx)(n.code,{children:'JSON-RPC\n{\n    "jsonrpc": "2.0",\n    "id": "1",\n    "method": "devnet_postmanFlush"\n}\n'})}),"\n",(0,s.jsxs)(n.p,{children:["Goes through the newly enqueued messages since the last flush, consuming and sending them from L1 to L2 and from L2 to L1. Use it for end-to-end testing. Requires no body. Optionally, set the ",(0,s.jsx)(n.code,{children:"dry_run"})," boolean flag to just see the result of flushing, without actually triggering it:"]}),"\n",(0,s.jsx)(n.pre,{children:(0,s.jsx)(n.code,{children:"POST /postman/flush\n"})}),"\n",(0,s.jsx)(n.pre,{children:(0,s.jsx)(n.code,{className:"language-js",children:'{ "dry_run": true }\n'})}),"\n",(0,s.jsx)(n.pre,{children:(0,s.jsx)(n.code,{children:'JSON-RPC\n{\n    "jsonrpc": "2.0",\n    "id": "1",\n    "method": "devnet_postmanFlush",\n    "params": {\n      "dry_run": true\n    }\n}\n'})}),"\n",(0,s.jsxs)(n.p,{children:["A running L1 node is required if ",(0,s.jsx)(n.code,{children:"dry_run"})," is not set."]}),"\n",(0,s.jsx)(n.admonition,{title:"Dumping and Loading",type:"info",children:(0,s.jsxs)(n.p,{children:["Flushing is not dumpable, meaning that, if you've enabled dumping, a flushing event will not itself be re-executed on loading. This is because it produces L2 messaging events that are themselves dumped. No L1-side actions are dumped, you need to take care of those yourself. Read more about dumping ",(0,s.jsx)(n.a,{href:"./dump-load-restart#dumping",children:"here"}),"."]})}),"\n",(0,s.jsx)(n.h2,{id:"disclaimer",children:"Disclaimer"}),"\n",(0,s.jsxs)(n.p,{children:["This method of L1-L2 communication testing differs from how Starknet mainnet and testnets work. Taking ",(0,s.jsx)(n.a,{href:"https://github.com/MikeSpa/starknet-test/blob/6a68d033cd7ddb5df937154f860f1c06174e6860/L1L2Example.sol#L46",children:(0,s.jsx)(n.strong,{children:"L1L2Example.sol"})})," (originally from Starknet documentation, no longer available there):"]}),"\n",(0,s.jsx)(n.pre,{children:(0,s.jsx)(n.code,{className:"language-solidity",children:"constructor(IStarknetCore starknetCore_) public {\n    starknetCore = starknetCore_;\n}\n"})}),"\n",(0,s.jsxs)(n.p,{children:["The constructor takes an ",(0,s.jsx)(n.code,{children:"IStarknetCore"})," contract as argument, however for Devnet's L1-L2 communication testing, this has to be replaced with the logic in ",(0,s.jsx)(n.a,{href:"https://github.com/starkware-libs/cairo-lang/blob/master/src/starkware/starknet/testing/MockStarknetMessaging.sol",children:(0,s.jsx)(n.strong,{children:"MockStarknetMessaging.sol"})}),":"]}),"\n",(0,s.jsx)(n.pre,{children:(0,s.jsx)(n.code,{className:"language-solidity",children:"constructor(MockStarknetMessaging mockStarknetMessaging_) public {\n    starknetCore = mockStarknetMessaging_;\n}\n"})}),"\n",(0,s.jsx)(n.h2,{id:"mock-transactions",children:"Mock transactions"}),"\n",(0,s.jsx)(n.h3,{id:"l1-l2",children:"L1->L2"}),"\n",(0,s.jsx)(n.admonition,{type:"note",children:(0,s.jsxs)(n.p,{children:["A running L1 node is ",(0,s.jsx)(n.strong,{children:"not"})," required for this operation."]})}),"\n",(0,s.jsxs)(n.p,{children:["Sends a mock transactions to L2, as if coming from L1, without the need for running L1. The deployed L2 contract address ",(0,s.jsx)(n.code,{children:"l2_contract_address"})," and ",(0,s.jsx)(n.code,{children:"entry_point_selector"})," must be valid, otherwise a new block will not be created."]}),"\n",(0,s.jsxs)(n.p,{children:["Normally ",(0,s.jsx)(n.code,{children:"nonce"})," is calculated by the L1 Starknet contract and it is used in L1 and L2. In this case, it needs to be provided manually."]}),"\n",(0,s.jsx)(n.pre,{children:(0,s.jsx)(n.code,{children:"POST /postman/send_message_to_l2\n"})}),"\n",(0,s.jsx)(n.p,{children:"Request:"}),"\n",(0,s.jsx)(n.pre,{children:(0,s.jsx)(n.code,{className:"language-js",children:'{\n    "l2_contract_address": "0x00285ddb7e5c777b310d806b9b2a0f7c7ba0a41f12b420219209d97a3b7f25b2",\n    "entry_point_selector": "0xC73F681176FC7B3F9693986FD7B14581E8D540519E27400E88B8713932BE01",\n    "l1_contract_address": "0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512",\n    "payload": [\n      "0x1",\n      "0x2"\n    ],\n    "paid_fee_on_l1": "0x123456abcdef",\n    "nonce":"0x0"\n}\n'})}),"\n",(0,s.jsx)(n.pre,{children:(0,s.jsx)(n.code,{children:'JSON-RPC\n{\n    "jsonrpc": "2.0",\n    "id": "1",\n    "method": "devnet_postmanSendMessageToL2",\n    "params": {\n      "l2_contract_address": "0x00285ddb7e5c777b310d806b9b2a0f7c7ba0a41f12b420219209d97a3b7f25b2",\n      "entry_point_selector": "0xC73F681176FC7B3F9693986FD7B14581E8D540519E27400E88B8713932BE01",\n      "l1_contract_address": "0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512",\n      "payload": [\n        "0x1",\n        "0x2"\n      ],\n      "paid_fee_on_l1": "0x123456abcdef",\n      "nonce":"0x0"\n  }\n}\n'})}),"\n",(0,s.jsx)(n.p,{children:"Response:"}),"\n",(0,s.jsx)(n.pre,{children:(0,s.jsx)(n.code,{className:"language-js",children:'{ "transaction_hash": "0x0548c761a9fd5512782998b2da6f44c42bf78fb88c3794eea330a91c9abb10bb" }\n'})}),"\n",(0,s.jsx)(n.h3,{id:"l2-l1",children:"L2->L1"}),"\n",(0,s.jsxs)(n.p,{children:["Sends a mock transaction from L2 to L1. The deployed L2 contract address ",(0,s.jsx)(n.code,{children:"from_address"})," and ",(0,s.jsx)(n.code,{children:"to_address"})," must be valid."]}),"\n",(0,s.jsx)(n.p,{children:"It is a mock message, but only in the sense that you are mocking an L2 contract's action, which would normally be triggered by invoking the contract via a transaction. So keep in mind the following:"}),"\n",(0,s.jsx)(n.admonition,{type:"note",children:(0,s.jsx)(n.p,{children:"A running L1 node is required for this operation."})}),"\n",(0,s.jsx)(n.pre,{children:(0,s.jsx)(n.code,{children:"POST /postman/consume_message_from_l2\n"})}),"\n",(0,s.jsx)(n.p,{children:"Request:"}),"\n",(0,s.jsx)(n.pre,{children:(0,s.jsx)(n.code,{className:"language-js",children:'{\n    "from_address": "0x00285ddb7e5c777b310d806b9b2a0f7c7ba0a41f12b420219209d97a3b7f25b2",\n    "to_address": "0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512",\n    "payload": ["0x0", "0x1", "0x3e8"],\n}\n'})}),"\n",(0,s.jsx)(n.pre,{children:(0,s.jsx)(n.code,{children:'JSON-RPC\n{\n    "jsonrpc": "2.0",\n    "id": "1",\n    "method": "devnet_postmanConsumeMessageFromL2",\n    "params": {\n      "from_address": "0x00285ddb7e5c777b310d806b9b2a0f7c7ba0a41f12b420219209d97a3b7f25b2",\n      "to_address": "0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512",\n      "payload": ["0x0", "0x1", "0x3e8"],\n  }\n}\n'})}),"\n",(0,s.jsx)(n.p,{children:"Response:"}),"\n",(0,s.jsx)(n.pre,{children:(0,s.jsx)(n.code,{className:"language-js",children:'{"message_hash": "0xae14f241131b524ac8d043d9cb4934253ac5c5589afef19f0d761816a9c7e26d"}\n'})})]})}function h(e={}){const{wrapper:n}={...(0,r.R)(),...e.components};return n?(0,s.jsx)(n,{...e,children:(0,s.jsx)(l,{...e})}):l(e)}},8453:(e,n,t)=>{t.d(n,{R:()=>o,x:()=>i});var s=t(6540);const r={},a=s.createContext(r);function o(e){const n=s.useContext(a);return s.useMemo((function(){return"function"==typeof e?e(n):{...n,...e}}),[n,e])}function i(e){let n;return n=e.disableParentContext?"function"==typeof e.components?e.components(r):e.components||r:o(e.components),s.createElement(a.Provider,{value:n},e.children)}}}]);
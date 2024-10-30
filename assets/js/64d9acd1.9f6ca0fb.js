"use strict";(self.webpackChunkwebsite=self.webpackChunkwebsite||[]).push([[4213],{5211:(e,n,t)=>{t.r(n),t.d(n,{assets:()=>a,contentTitle:()=>s,default:()=>p,frontMatter:()=>r,metadata:()=>d,toc:()=>i});var o=t(4848),c=t(8453);const r={},s="Predeployed contracts",d={id:"predeployed",title:"Predeployed contracts",description:"Devnet predeploys a UDC, an ERC20 (fee token) contract and a set of predeployed funded accounts.",source:"@site/versioned_docs/version-0.2.0/predeployed.md",sourceDirName:".",slug:"/predeployed",permalink:"/starknet-devnet-rs/docs/0.2.0/predeployed",draft:!1,unlisted:!1,editUrl:"https://github.com/0xSpaceShard/starknet-devnet-rs/blob/master/website/versioned_docs/version-0.2.0/predeployed.md",tags:[],version:"0.2.0",frontMatter:{},sidebar:"docSidebar",previous:{title:"L1-L2 interaction via Postman",permalink:"/starknet-devnet-rs/docs/0.2.0/postman"},next:{title:"Restrictive mode",permalink:"/starknet-devnet-rs/docs/0.2.0/restrictive"}},a={},i=[{value:"Predeployed account preservation",id:"predeployed-account-preservation",level:2},{value:"Account class selection",id:"account-class-selection",level:2},{value:"Deploying an undeclared account",id:"deploying-an-undeclared-account",level:2},{value:"How to get predeployment info?",id:"how-to-get-predeployment-info",level:2}];function l(e){const n={a:"a",admonition:"admonition",code:"code",h1:"h1",h2:"h2",li:"li",p:"p",pre:"pre",ul:"ul",...(0,c.R)(),...e.components};return(0,o.jsxs)(o.Fragment,{children:[(0,o.jsx)(n.h1,{id:"predeployed-contracts",children:"Predeployed contracts"}),"\n",(0,o.jsxs)(n.p,{children:["Devnet predeploys a ",(0,o.jsx)(n.a,{href:"https://docs.openzeppelin.com/contracts-cairo/0.6.1/udc",children:"UDC"}),", an ",(0,o.jsx)(n.a,{href:"https://docs.openzeppelin.com/contracts-cairo/0.8.1/erc20",children:"ERC20 (fee token)"})," contract and a set of predeployed funded accounts."]}),"\n",(0,o.jsxs)(n.p,{children:["The set of accounts can be controlled via ",(0,o.jsx)(n.a,{href:"./running/cli",children:"CLI options"}),": ",(0,o.jsx)(n.code,{children:"--accounts <NUMBER_OF>"}),", ",(0,o.jsx)(n.code,{children:"--initial-balance <WEI>"}),", ",(0,o.jsx)(n.code,{children:"--seed <VALUE>"}),"."]}),"\n",(0,o.jsx)(n.h2,{id:"predeployed-account-preservation",children:"Predeployed account preservation"}),"\n",(0,o.jsxs)(n.admonition,{type:"note",children:[(0,o.jsxs)(n.p,{children:["Once you shut down your Devnet, the predeployed account you used ceases to exist. This may be a problem with tools such as ",(0,o.jsx)(n.code,{children:"starkli"})," which hardcode your account details in a local file. One option then is to delete your account entry from ",(0,o.jsx)(n.code,{children:"starkli"}),"'s account file. Another option is to spawn the same account on next Devnet startup. To do this, you can use:"]}),(0,o.jsxs)(n.ul,{children:["\n",(0,o.jsxs)(n.li,{children:["the ",(0,o.jsx)(n.code,{children:"--seed <VALUE>"})," CLI option which always predeploys the same set of accounts if using the same ",(0,o.jsx)(n.code,{children:"<VALUE>"})," (the seed is logged on startup)"]}),"\n",(0,o.jsxs)(n.li,{children:["the ",(0,o.jsx)(n.a,{href:"./dump-load-restart",children:"dump and load feature"})]}),"\n"]})]}),"\n",(0,o.jsx)(n.h2,{id:"account-class-selection",children:"Account class selection"}),"\n",(0,o.jsx)(n.p,{children:"Choose between predeploying Cairo 0 (OpenZeppelin 0.5.1) or Cairo 1 (default; OpenZeppelin 0.8.1) accounts by using:"}),"\n",(0,o.jsx)(n.pre,{children:(0,o.jsx)(n.code,{children:"--account-class [cairo0 | cairo1]\n"})}),"\n",(0,o.jsxs)(n.p,{children:["Alternatively, provide a path to the ",(0,o.jsx)(n.a,{href:"https://github.com/starkware-libs/cairo#compiling-and-running-cairo-files",children:"Sierra artifact"})," of your custom account using:"]}),"\n",(0,o.jsx)(n.pre,{children:(0,o.jsx)(n.code,{children:"--account-class-custom <SIERRA_PATH>\n"})}),"\n",(0,o.jsx)(n.h2,{id:"deploying-an-undeclared-account",children:"Deploying an undeclared account"}),"\n",(0,o.jsxs)(n.p,{children:["If you want to deploy an instance of an account contract class not predeclared on Devnet, you can use ",(0,o.jsx)(n.a,{href:"./forking",children:"forking"}),". Just fork an origin network which has the needed class already declared, e.g. the Sepolia testnet. Why? Because new versions of wallets like ArgentX and Braavos tend to be declared on testnet/mainnet soon after release."]}),"\n",(0,o.jsx)(n.h2,{id:"how-to-get-predeployment-info",children:"How to get predeployment info?"}),"\n",(0,o.jsxs)(n.p,{children:["The information on predeployed contracts is logged on Devnet startup. Predeployed accounts can be retrieved in JSON format by sending a ",(0,o.jsx)(n.code,{children:"GET"})," request to ",(0,o.jsx)(n.code,{children:"/predeployed_accounts"}),". With the additional query parameter ",(0,o.jsx)(n.code,{children:"with_balance=true"}),", WEI and FRI balances at the pending state will be provided:"]}),"\n",(0,o.jsx)(n.pre,{children:(0,o.jsx)(n.code,{children:"GET /predeployed_accounts?[with_balance=true]\n"})}),"\n",(0,o.jsx)(n.p,{children:"Alternatively, send a JSON-RPC request:"}),"\n",(0,o.jsx)(n.pre,{children:(0,o.jsx)(n.code,{children:'JSON-RPC\n{\n    "jsonrpc": "2.0",\n    "id": "1",\n    "method": "devnet_getPredeployedAccounts"\n}\n'})}),"\n",(0,o.jsx)(n.pre,{children:(0,o.jsx)(n.code,{children:'JSON-RPC\n{\n    "jsonrpc": "2.0",\n    "id": "1",\n    "method": "devnet_getPredeployedAccounts",\n    "params": {\n        // optional; defaults to false\n        "with_balance": true | false\n    }\n}\n'})})]})}function p(e={}){const{wrapper:n}={...(0,c.R)(),...e.components};return n?(0,o.jsx)(n,{...e,children:(0,o.jsx)(l,{...e})}):l(e)}},8453:(e,n,t)=>{t.d(n,{R:()=>s,x:()=>d});var o=t(6540);const c={},r=o.createContext(c);function s(e){const n=o.useContext(r);return o.useMemo((function(){return"function"==typeof e?e(n):{...n,...e}}),[n,e])}function d(e){let n;return n=e.disableParentContext?"function"==typeof e.components?e.components(c):e.components||c:s(e.components),o.createElement(r.Provider,{value:n},e.children)}}}]);
"use strict";(self.webpackChunkwebsite=self.webpackChunkwebsite||[]).push([[1978],{5434:(e,t,n)=>{n.r(t),n.d(t,{assets:()=>c,contentTitle:()=>a,default:()=>h,frontMatter:()=>i,metadata:()=>o,toc:()=>l});var s=n(4848),r=n(8453);const i={},a="Gas price modification",o={id:"gas",title:"Gas price modification",description:"The devnetsetGasPrice RPC method allows users to modify the current gas prices on a running Devnet. This feature is particularly useful for testing purposes and for adjustments needed after forking to align with the forked network's gas prices. All parameters are optional, allowing you to choose which ones you want to set. A boolean flag generateblock indicates whether a new block should be generated immediately after setting the gas prices.",source:"@site/versioned_docs/version-0.3.0-rc.0/gas.md",sourceDirName:".",slug:"/gas",permalink:"/starknet-devnet/docs/gas",draft:!1,unlisted:!1,editUrl:"https://github.com/0xSpaceShard/starknet-devnet/blob/master/website/versioned_docs/version-0.3.0-rc.0/gas.md",tags:[],version:"0.3.0-rc.0",frontMatter:{},sidebar:"docSidebar",previous:{title:"Forking",permalink:"/starknet-devnet/docs/forking"},next:{title:"Historic state support",permalink:"/starknet-devnet/docs/historic-state"}},c={},l=[{value:"Explanation",id:"explanation",level:2},{value:"generate_block",id:"generate_block",level:3},{value:"JSON-RPC Request",id:"json-rpc-request",level:2},{value:"Response",id:"response",level:2}];function d(e){const t={code:"code",h1:"h1",h2:"h2",h3:"h3",li:"li",p:"p",pre:"pre",ul:"ul",...(0,r.R)(),...e.components};return(0,s.jsxs)(s.Fragment,{children:[(0,s.jsx)(t.h1,{id:"gas-price-modification",children:"Gas price modification"}),"\n",(0,s.jsxs)(t.p,{children:["The ",(0,s.jsx)(t.code,{children:"devnet_setGasPrice"})," RPC method allows users to modify the current gas prices on a running Devnet. This feature is particularly useful for testing purposes and for adjustments needed after forking to align with the forked network's gas prices. All parameters are optional, allowing you to choose which ones you want to set. A boolean flag ",(0,s.jsx)(t.code,{children:"generate_block"})," indicates whether a new block should be generated immediately after setting the gas prices."]}),"\n",(0,s.jsx)(t.h2,{id:"explanation",children:"Explanation"}),"\n",(0,s.jsx)(t.p,{children:"The modified gas prices take effect starting with the next block that is generated."}),"\n",(0,s.jsx)(t.h3,{id:"generate_block",children:"generate_block"}),"\n",(0,s.jsxs)(t.ul,{children:["\n",(0,s.jsxs)(t.li,{children:["When set to ",(0,s.jsx)(t.code,{children:"true"}),", a new block will be generated immediately after the gas prices are set. This ensures that the changes take effect right away and are reflected in the devnet state without waiting for the next block generation."]}),"\n",(0,s.jsxs)(t.li,{children:["When set to ",(0,s.jsx)(t.code,{children:"false"})," (or omitted), the gas prices will be set, but the changes will not be immediately committed to the devnet state until the next block is generated through the usual block generation process."]}),"\n"]}),"\n",(0,s.jsx)(t.h2,{id:"json-rpc-request",children:"JSON-RPC Request"}),"\n",(0,s.jsx)(t.p,{children:"The following JSON-RPC request can be used to set gas prices:"}),"\n",(0,s.jsx)(t.pre,{children:(0,s.jsx)(t.code,{children:'JSON-RPC\n{\n    "jsonrpc": "2.0",\n    "id": "1",\n    "method": "setGasPrice"\n    "params": {\n        "gas_price_wei": 1000000,\n        "data_gas_price_wei": 10000,\n        "gas_price_fri": 10000,\n        "data_gas_price_fri": 10000,\n        "generate_block": false,\n    }\n}\n'})}),"\n",(0,s.jsx)(t.h2,{id:"response",children:"Response"}),"\n",(0,s.jsx)(t.p,{children:"The expected response from the server will mirror the request gas parameters, confirming the modification of gas prices:"}),"\n",(0,s.jsx)(t.pre,{children:(0,s.jsx)(t.code,{children:'{\n    "gas_price_wei": 1000000,\n    "data_gas_price_wei": 10000,\n    "gas_price_fri": 10000,\n    "data_gas_price_fri": 10000,\n}\n'})})]})}function h(e={}){const{wrapper:t}={...(0,r.R)(),...e.components};return t?(0,s.jsx)(t,{...e,children:(0,s.jsx)(d,{...e})}):d(e)}},8453:(e,t,n)=>{n.d(t,{R:()=>a,x:()=>o});var s=n(6540);const r={},i=s.createContext(r);function a(e){const t=s.useContext(i);return s.useMemo((function(){return"function"==typeof e?e(t):{...t,...e}}),[t,e])}function o(e){let t;return t=e.disableParentContext?"function"==typeof e.components?e.components(r):e.components||r:a(e.components),s.createElement(i.Provider,{value:t},e.children)}}}]);
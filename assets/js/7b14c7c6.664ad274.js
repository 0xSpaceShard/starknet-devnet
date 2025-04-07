"use strict";(self.webpackChunkwebsite=self.webpackChunkwebsite||[]).push([[1454],{2113:(n,e,t)=>{t.r(e),t.d(e,{assets:()=>c,contentTitle:()=>a,default:()=>p,frontMatter:()=>i,metadata:()=>r,toc:()=>d});var o=t(4848),s=t(8453);const i={},a="Account impersonation",r={id:"account-impersonation",title:"Account impersonation",description:"This page is about account impersonation. To read about account class selection and deployment, click here.",source:"@site/versioned_docs/version-0.3.0/account-impersonation.md",sourceDirName:".",slug:"/account-impersonation",permalink:"/starknet-devnet/docs/account-impersonation",draft:!1,unlisted:!1,editUrl:"https://github.com/0xSpaceShard/starknet-devnet/blob/master/website/versioned_docs/version-0.3.0/account-impersonation.md",tags:[],version:"0.3.0",frontMatter:{},sidebar:"docSidebar",previous:{title:"API",permalink:"/starknet-devnet/docs/api"},next:{title:"Account balance",permalink:"/starknet-devnet/docs/balance"}},c={},d=[{value:"Introduction",id:"introduction",level:2},{value:"Disabling impersonation",id:"disabling-impersonation",level:2},{value:"API",id:"api",level:2},{value:"devnet_impersonateAccount",id:"devnet_impersonateaccount",level:3},{value:"devnet_stopImpersonateAccount",id:"devnet_stopimpersonateaccount",level:3},{value:"devnet_autoImpersonate",id:"devnet_autoimpersonate",level:3},{value:"devnet_stopAutoImpersonate",id:"devnet_stopautoimpersonate",level:3}];function l(n){const e={a:"a",admonition:"admonition",code:"code",h1:"h1",h2:"h2",h3:"h3",li:"li",p:"p",pre:"pre",strong:"strong",ul:"ul",...(0,s.R)(),...n.components};return(0,o.jsxs)(o.Fragment,{children:[(0,o.jsx)(e.h1,{id:"account-impersonation",children:"Account impersonation"}),"\n",(0,o.jsx)(e.admonition,{type:"info",children:(0,o.jsxs)(e.p,{children:["This page is about account impersonation. To read about account class selection and deployment, click ",(0,o.jsx)(e.a,{href:"./predeployed",children:"here"}),"."]})}),"\n",(0,o.jsx)(e.h2,{id:"introduction",children:"Introduction"}),"\n",(0,o.jsxs)(e.p,{children:["Devnet allows you to use impersonated account from mainnet/testnet. This means that a transaction sent from an impersonated account will not fail with an invalid signature error. In the general case, a transaction sent with an account that is not in the local state fails with the aforementioned error. For impersonation to work, Devnet needs to be run in ",(0,o.jsx)(e.a,{href:"/starknet-devnet/docs/forking",children:"forking mode"}),"."]}),"\n",(0,o.jsx)(e.admonition,{title:"Caveat",type:"warning",children:(0,o.jsxs)(e.ul,{children:["\n",(0,o.jsxs)(e.li,{children:["Only ",(0,o.jsx)(e.code,{children:"INVOKE"})," and ",(0,o.jsx)(e.code,{children:"DECLARE"})," transactions are supported. ",(0,o.jsx)(e.code,{children:"DEPLOY_ACCOUNT"})," transaction is not supported, but you can create an ",(0,o.jsx)(e.code,{children:"INVOKE"})," transaction to UDC."]}),"\n",(0,o.jsx)(e.li,{children:"Overall fee, for transactions sent with an impersonated account, will be lower compared to normal transactions. The reason is that validation part is skipped."}),"\n",(0,o.jsxs)(e.li,{children:["The most common way of sending a transaction is via starknet-rs/starknet.js or starkli. Trying to send with an account that ",(0,o.jsx)(e.strong,{children:"does not"})," exist even in the origin network will return an error:","\n",(0,o.jsxs)(e.ul,{children:["\n",(0,o.jsxs)(e.li,{children:["In transaction construction, if account nonce is not hardcoded, Devnet is queried and returns ",(0,o.jsx)(e.code,{children:"ContractNotFound"}),"."]}),"\n",(0,o.jsxs)(e.li,{children:["Otherwise the nonce fetching part is skipped and ",(0,o.jsx)(e.code,{children:"InsufficientAccountBalance"})," is returned."]}),"\n"]}),"\n"]}),"\n"]})}),"\n",(0,o.jsx)(e.h2,{id:"disabling-impersonation",children:"Disabling impersonation"}),"\n",(0,o.jsxs)(e.p,{children:["Click ",(0,o.jsx)(e.a,{href:"/starknet-devnet/docs/restrictive",children:"here"})," to learn how to disable account impersonation."]}),"\n",(0,o.jsx)(e.h2,{id:"api",children:"API"}),"\n",(0,o.jsx)(e.p,{children:"Account impersonation follows JSON-RPC method specification. Each method returns an empty response:"}),"\n",(0,o.jsx)(e.h3,{id:"devnet_impersonateaccount",children:"devnet_impersonateAccount"}),"\n",(0,o.jsx)(e.p,{children:"Impersonates a specific account address nonexistent in the local state."}),"\n",(0,o.jsx)(e.pre,{children:(0,o.jsx)(e.code,{className:"language-js",children:'{\n    "jsonrpc": "2.0",\n    "id": "1",\n    "method": "devnet_impersonateAccount",\n    "params": {\n        "account_address": "0x49D36570D4E46F48E99674BD3FCC84644DDD6B96F7C741B1562B82F9E004DC7"\n    }\n}\n'})}),"\n",(0,o.jsx)(e.h3,{id:"devnet_stopimpersonateaccount",children:"devnet_stopImpersonateAccount"}),"\n",(0,o.jsx)(e.p,{children:"Stops the impersonation of an account previously marked for impersonation."}),"\n",(0,o.jsx)(e.pre,{children:(0,o.jsx)(e.code,{className:"language-js",children:'{\n    "jsonrpc": "2.0",\n    "id": "1",\n    "method": "devnet_stopImpersonateAccount",\n    "params": {\n        "account_address": "0x49D36570D4E46F48E99674BD3FCC84644DDD6B96F7C741B1562B82F9E004DC7"\n    }\n}\n'})}),"\n",(0,o.jsx)(e.h3,{id:"devnet_autoimpersonate",children:"devnet_autoImpersonate"}),"\n",(0,o.jsx)(e.p,{children:"Enables automatic account impersonation. Every account that does not exist in the local state will be impersonated."}),"\n",(0,o.jsx)(e.pre,{children:(0,o.jsx)(e.code,{className:"language-js",children:'{\n    "jsonrpc": "2.0",\n    "id": "1",\n    "method": "devnet_autoImpersonate",\n    "params": {}\n}\n'})}),"\n",(0,o.jsx)(e.h3,{id:"devnet_stopautoimpersonate",children:"devnet_stopAutoImpersonate"}),"\n",(0,o.jsxs)(e.p,{children:["Stops the effect of ",(0,o.jsx)(e.a,{href:"#devnet_autoimpersonate",children:"automatic impersonation"}),"."]}),"\n",(0,o.jsx)(e.pre,{children:(0,o.jsx)(e.code,{className:"language-js",children:'{\n    "jsonrpc": "2.0",\n    "id": "1",\n    "method": "devnet_stopAutoImpersonate",\n    "params": {}\n}\n'})})]})}function p(n={}){const{wrapper:e}={...(0,s.R)(),...n.components};return e?(0,o.jsx)(e,{...n,children:(0,o.jsx)(l,{...n})}):l(n)}},8453:(n,e,t)=>{t.d(e,{R:()=>a,x:()=>r});var o=t(6540);const s={},i=o.createContext(s);function a(n){const e=o.useContext(i);return o.useMemo((function(){return"function"==typeof n?n(e):{...e,...n}}),[e,n])}function r(n){let e;return e=n.disableParentContext?"function"==typeof n.components?n.components(s):n.components||s:a(n.components),o.createElement(i.Provider,{value:e},n.children)}}}]);
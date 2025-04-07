"use strict";(self.webpackChunkwebsite=self.webpackChunkwebsite||[]).push([[1217],{3107:(e,n,t)=>{t.r(n),t.d(n,{assets:()=>a,contentTitle:()=>c,default:()=>u,frontMatter:()=>r,metadata:()=>s,toc:()=>d});var o=t(4848),i=t(8453);const r={},c="Forking",s={id:"forking",title:"Forking",description:"To interact with contracts deployed on mainnet or testnet, you can use forking. Simulate the origin and experiment with it locally, making no changes to the origin itself.",source:"@site/versioned_docs/version-0.3.0/forking.md",sourceDirName:".",slug:"/forking",permalink:"/starknet-devnet/docs/forking",draft:!1,unlisted:!1,editUrl:"https://github.com/0xSpaceShard/starknet-devnet/blob/master/website/versioned_docs/version-0.3.0/forking.md",tags:[],version:"0.3.0",frontMatter:{},sidebar:"docSidebar",previous:{title:"Examples",permalink:"/starknet-devnet/docs/examples"},next:{title:"Gas price modification",permalink:"/starknet-devnet/docs/gas"}},a={},d=[{value:"Account impersonation",id:"account-impersonation",level:2},{value:"Deploying an undeclared account",id:"deploying-an-undeclared-account",level:2},{value:"Checking forking status",id:"checking-forking-status",level:2}];function l(e){const n={a:"a",code:"code",h1:"h1",h2:"h2",p:"p",pre:"pre",...(0,i.R)(),...e.components};return(0,o.jsxs)(o.Fragment,{children:[(0,o.jsx)(n.h1,{id:"forking",children:"Forking"}),"\n",(0,o.jsx)(n.p,{children:"To interact with contracts deployed on mainnet or testnet, you can use forking. Simulate the origin and experiment with it locally, making no changes to the origin itself."}),"\n",(0,o.jsx)(n.pre,{children:(0,o.jsx)(n.code,{children:"$ starknet-devnet --fork-network <URL> [--fork-block <BLOCK_NUMBER>]\n"})}),"\n",(0,o.jsxs)(n.p,{children:["The value passed to ",(0,o.jsx)(n.code,{children:"--fork-network"})," should be the URL to a Starknet JSON-RPC API provider. Specifying a ",(0,o.jsx)(n.code,{children:"--fork-block"})," is optional; it defaults to the ",(0,o.jsx)(n.code,{children:'"latest"'})," block at the time of Devnet's start-up. All calls will first try Devnet's state and then fall back to the forking block."]}),"\n",(0,o.jsx)(n.h2,{id:"account-impersonation",children:"Account impersonation"}),"\n",(0,o.jsxs)(n.p,{children:[(0,o.jsx)(n.a,{href:"./account-impersonation",children:"Here"})," you can read more about acting as an account deployed on the origin."]}),"\n",(0,o.jsx)(n.h2,{id:"deploying-an-undeclared-account",children:"Deploying an undeclared account"}),"\n",(0,o.jsxs)(n.p,{children:[(0,o.jsx)(n.a,{href:"./predeployed#deploying-an-undeclared-account",children:"Here"})," you can read about deploying an account not declared on Devnet."]}),"\n",(0,o.jsx)(n.h2,{id:"checking-forking-status",children:"Checking forking status"}),"\n",(0,o.jsxs)(n.p,{children:["To see if your Devnet instance is using forking or not, ",(0,o.jsx)(n.a,{href:"./api#config-api",children:"fetch the current configuration"}),", and check the ",(0,o.jsx)(n.code,{children:"url"})," property of its ",(0,o.jsx)(n.code,{children:"fork_config"})," property. If Devnet is forked, this property contains the string of the origin URL specified on startup."]})]})}function u(e={}){const{wrapper:n}={...(0,i.R)(),...e.components};return n?(0,o.jsx)(n,{...e,children:(0,o.jsx)(l,{...e})}):l(e)}},8453:(e,n,t)=>{t.d(n,{R:()=>c,x:()=>s});var o=t(6540);const i={},r=o.createContext(i);function c(e){const n=o.useContext(r);return o.useMemo((function(){return"function"==typeof e?e(n):{...n,...e}}),[n,e])}function s(e){let n;return n=e.disableParentContext?"function"==typeof e.components?e.components(i):e.components||i:c(e.components),o.createElement(r.Provider,{value:n},e.children)}}}]);
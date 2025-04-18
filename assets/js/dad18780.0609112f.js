"use strict";(self.webpackChunkwebsite=self.webpackChunkwebsite||[]).push([[5496],{7725:(e,t,n)=>{n.r(t),n.d(t,{assets:()=>d,contentTitle:()=>a,default:()=>m,frontMatter:()=>r,metadata:()=>c,toc:()=>o});var i=n(4848),s=n(8453);const r={},a="Starknet time",c={id:"starknet-time",title:"Starknet time",description:"Block and state timestamp can be manipulated by setting the exact time or setting the time offset. By default, timestamp methods /settime, /increasetime and JSON-RPC methods devnetsetTime, devnetincreaseTime generate a new block. This can be changed for /settime (devnetsetTime) by setting the optional parameter generate_block to false. This skips immediate new block generation, but will use the specified timestamp whenever the next block is supposed to be generated.",source:"@site/versioned_docs/version-0.3.0/starknet-time.md",sourceDirName:".",slug:"/starknet-time",permalink:"/starknet-devnet/docs/0.3.0/starknet-time",draft:!1,unlisted:!1,editUrl:"https://github.com/0xSpaceShard/starknet-devnet/blob/master/website/versioned_docs/version-0.3.0/starknet-time.md",tags:[],version:"0.3.0",frontMatter:{},sidebar:"docSidebar",previous:{title:"Server config",permalink:"/starknet-devnet/docs/0.3.0/server-config"}},d={},o=[{value:"Set time",id:"set-time",level:2},{value:"Increase time",id:"increase-time",level:2},{value:"Start time",id:"start-time",level:2}];function l(e){const t={a:"a",admonition:"admonition",code:"code",h1:"h1",h2:"h2",p:"p",pre:"pre",...(0,s.R)(),...e.components};return(0,i.jsxs)(i.Fragment,{children:[(0,i.jsx)(t.h1,{id:"starknet-time",children:"Starknet time"}),"\n",(0,i.jsxs)(t.p,{children:["Block and state timestamp can be manipulated by setting the exact time or setting the time offset. By default, timestamp methods ",(0,i.jsx)(t.code,{children:"/set_time"}),", ",(0,i.jsx)(t.code,{children:"/increase_time"})," and ",(0,i.jsx)(t.code,{children:"JSON-RPC"})," methods ",(0,i.jsx)(t.code,{children:"devnet_setTime"}),", ",(0,i.jsx)(t.code,{children:"devnet_increaseTime"})," generate a new block. This can be changed for ",(0,i.jsx)(t.code,{children:"/set_time"})," (",(0,i.jsx)(t.code,{children:"devnet_setTime"}),") by setting the optional parameter ",(0,i.jsx)(t.code,{children:"generate_block"})," to ",(0,i.jsx)(t.code,{children:"false"}),". This skips immediate new block generation, but will use the specified timestamp whenever the next block is supposed to be generated."]}),"\n",(0,i.jsxs)(t.p,{children:["All values should be set in ",(0,i.jsx)(t.a,{href:"https://en.wikipedia.org/wiki/Unix_time",children:"Unix time seconds"}),". After ",(0,i.jsx)(t.a,{href:"#start-time",children:"startup"}),", the time progresses naturally."]}),"\n",(0,i.jsx)(t.h2,{id:"set-time",children:"Set time"}),"\n",(0,i.jsx)(t.p,{children:"The following sets the exact time and generates a new block:"}),"\n",(0,i.jsx)(t.pre,{children:(0,i.jsx)(t.code,{children:'POST /set_time\n{\n    "time": TIME_IN_SECONDS\n}\n'})}),"\n",(0,i.jsx)(t.pre,{children:(0,i.jsx)(t.code,{children:'JSON-RPC\n{\n    "jsonrpc": "2.0",\n    "id": "1",\n    "method": "devnet_setTime",\n    "params": {\n        "time": TIME_IN_SECONDS\n    }\n}\n'})}),"\n",(0,i.jsx)(t.p,{children:"The following doesn't generate a new block, but sets the exact time for the next generated block:"}),"\n",(0,i.jsx)(t.pre,{children:(0,i.jsx)(t.code,{children:'POST /set_time\n{\n    "time": TIME_IN_SECONDS,\n    "generate_block": false\n}\n'})}),"\n",(0,i.jsx)(t.pre,{children:(0,i.jsx)(t.code,{children:'JSON-RPC\n{\n    "jsonrpc": "2.0",\n    "id": "1",\n    "method": "devnet_setTime",\n    "params": {\n        "time": TIME_IN_SECONDS,\n        "generate_block": false\n    }\n}\n'})}),"\n",(0,i.jsx)(t.p,{children:"Warning: block time can be set in the past which might lead to unexpected behavior!"}),"\n",(0,i.jsx)(t.h2,{id:"increase-time",children:"Increase time"}),"\n",(0,i.jsx)(t.p,{children:"Increases the block timestamp by the provided amount and generates a new block. All subsequent blocks will keep this increment."}),"\n",(0,i.jsx)(t.pre,{children:(0,i.jsx)(t.code,{children:'POST /increase_time\n{\n    "time": TIME_IN_SECONDS\n}\n'})}),"\n",(0,i.jsx)(t.pre,{children:(0,i.jsx)(t.code,{children:'JSON-RPC\n{\n    "jsonrpc": "2.0",\n    "id": "1",\n    "method": "devnet_increaseTime",\n    "params": {\n        "time": TIME_IN_SECONDS\n    }\n}\n'})}),"\n",(0,i.jsx)(t.admonition,{title:"Increment example",type:"note",children:(0,i.jsxs)(t.p,{children:["Imagine a block is generated with timestamp ",(0,i.jsx)(t.code,{children:"T1"}),", some time passes (let's call this interval ",(0,i.jsx)(t.code,{children:"T_passed"}),"), and you call increase_time with ",(0,i.jsx)(t.code,{children:"T_inc"})," as the argument, which immediately mines a new block at ",(0,i.jsx)(t.code,{children:"T2"}),". ",(0,i.jsx)(t.code,{children:"T2"})," should equal ",(0,i.jsx)(t.code,{children:"T1 + T_passed + T_inc"}),"."]})}),"\n",(0,i.jsx)(t.h2,{id:"start-time",children:"Start time"}),"\n",(0,i.jsxs)(t.p,{children:["Devnet's starting timestamp can be defined via CLI by providing a positive value of ",(0,i.jsx)(t.a,{href:"https://en.wikipedia.org/wiki/Unix_time",children:"Unix time seconds"})," to ",(0,i.jsx)(t.code,{children:"--start-time"}),":"]}),"\n",(0,i.jsx)(t.pre,{children:(0,i.jsx)(t.code,{children:"$ starknet-devnet --start-time <SECONDS>\n"})}),"\n",(0,i.jsx)(t.p,{children:"If provided, this timestamp shall be used in mining the genesis block. The default value is the current Unix time."})]})}function m(e={}){const{wrapper:t}={...(0,s.R)(),...e.components};return t?(0,i.jsx)(t,{...e,children:(0,i.jsx)(l,{...e})}):l(e)}},8453:(e,t,n)=>{n.d(t,{R:()=>a,x:()=>c});var i=n(6540);const s={},r=i.createContext(s);function a(e){const t=i.useContext(r);return i.useMemo((function(){return"function"==typeof e?e(t):{...t,...e}}),[t,e])}function c(e){let t;return t=e.disableParentContext?"function"==typeof e.components?e.components(s):e.components||s:a(e.components),i.createElement(r.Provider,{value:t},e.children)}}}]);
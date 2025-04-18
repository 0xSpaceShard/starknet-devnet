"use strict";(self.webpackChunkwebsite=self.webpackChunkwebsite||[]).push([[8159],{5846:(e,n,r)=>{r.r(n),r.d(n,{assets:()=>c,contentTitle:()=>o,default:()=>h,frontMatter:()=>t,metadata:()=>a,toc:()=>d});var i=r(4848),s=r(8453);const t={sidebar_position:2.3},o="CLI options",a={id:"running/cli",title:"CLI options",description:"Configure your Devnet instance by specifying CLI parameters on startup. To read more about HTTP and logging configuration, check out the server config page.",source:"@site/versioned_docs/version-0.3.0/running/cli.md",sourceDirName:"running",slug:"/running/cli",permalink:"/starknet-devnet/docs/0.3.0/running/cli",draft:!1,unlisted:!1,editUrl:"https://github.com/0xSpaceShard/starknet-devnet/blob/master/website/versioned_docs/version-0.3.0/running/cli.md",tags:[],version:"0.3.0",sidebarPosition:2.3,frontMatter:{sidebar_position:2.3},sidebar:"docSidebar",previous:{title:"Run with Docker",permalink:"/starknet-devnet/docs/0.3.0/running/docker"},next:{title:"API",permalink:"/starknet-devnet/docs/0.3.0/api"}},c={},d=[{value:"Help",id:"help",level:2},{value:"Environment variables",id:"environment-variables",level:2},{value:"Precedence",id:"precedence",level:3},{value:"Docker",id:"docker",level:3},{value:"Load configuration from a file",id:"load-configuration-from-a-file",level:2},{value:"Docker",id:"docker-1",level:3}];function l(e){const n={a:"a",code:"code",h1:"h1",h2:"h2",h3:"h3",p:"p",pre:"pre",...(0,s.R)(),...e.components};return(0,i.jsxs)(i.Fragment,{children:[(0,i.jsx)(n.h1,{id:"cli-options",children:"CLI options"}),"\n",(0,i.jsxs)(n.p,{children:["Configure your Devnet instance by specifying CLI parameters on startup. To read more about HTTP and logging configuration, check out the ",(0,i.jsx)(n.a,{href:"../server-config",children:"server config"})," page."]}),"\n",(0,i.jsx)(n.h2,{id:"help",children:"Help"}),"\n",(0,i.jsx)(n.p,{children:"Check out all the options with:"}),"\n",(0,i.jsx)(n.pre,{children:(0,i.jsx)(n.code,{children:"$ starknet-devnet --help\n"})}),"\n",(0,i.jsx)(n.p,{children:"Or if using dockerized Devnet:"}),"\n",(0,i.jsx)(n.pre,{children:(0,i.jsx)(n.code,{children:"$ docker run --rm shardlabs/starknet-devnet-rs --help\n"})}),"\n",(0,i.jsx)(n.h2,{id:"environment-variables",children:"Environment variables"}),"\n",(0,i.jsx)(n.p,{children:"Every CLI option can also be specified via an environment variable:"}),"\n",(0,i.jsx)(n.pre,{children:(0,i.jsx)(n.code,{children:"$ <VAR1>=<VALUE> <VAR2>=<VALUE> starknet-devnet\n"})}),"\n",(0,i.jsxs)(n.p,{children:["To see the exact variable names, use ",(0,i.jsx)(n.a,{href:"#help",children:(0,i.jsx)(n.code,{children:"--help"})}),"."]}),"\n",(0,i.jsx)(n.h3,{id:"precedence",children:"Precedence"}),"\n",(0,i.jsx)(n.p,{children:"If both a CLI argument and an environment variable are passed for a parameter, the CLI argument takes precedence. If none are provided, the default value is used. E.g. if running Devnet with the following command, seed value 42 will be used:"}),"\n",(0,i.jsx)(n.pre,{children:(0,i.jsx)(n.code,{children:"$ SEED=10 starknet-devnet --seed 42\n"})}),"\n",(0,i.jsx)(n.h3,{id:"docker",children:"Docker"}),"\n",(0,i.jsx)(n.p,{children:"If using dockerized Devnet, specify the variables like this:"}),"\n",(0,i.jsx)(n.pre,{children:(0,i.jsx)(n.code,{children:"$ docker run \\\n    -e <VAR1>=<VALUE> \\\n    -e <VAR2>=<VALUE> \\\n    ... \\\n    shardlabs/starknet-devnet-rs\n"})}),"\n",(0,i.jsx)(n.h2,{id:"load-configuration-from-a-file",children:"Load configuration from a file"}),"\n",(0,i.jsxs)(n.p,{children:["If providing many configuration parameters in a single command becomes cumbersome, consider loading them from a file. By relying on ",(0,i.jsx)(n.a,{href:"#environment-variables",children:"environment variables"}),", prepare your configuration in a file like this:"]}),"\n",(0,i.jsx)(n.pre,{children:(0,i.jsx)(n.code,{className:"language-bash",children:"export SEED=42\nexport ACCOUNTS=3\n...\n"})}),"\n",(0,i.jsxs)(n.p,{children:["Assuming the file is called ",(0,i.jsx)(n.code,{children:".my-env-file"}),", then run:"]}),"\n",(0,i.jsx)(n.pre,{children:(0,i.jsx)(n.code,{className:"language-bash",children:"$ source .my-env-file && starknet-devnet\n"})}),"\n",(0,i.jsx)(n.p,{children:"To run in a subshell and prevent environment pollution (i.e. to unset the variables after Devnet exits), use parentheses:"}),"\n",(0,i.jsx)(n.pre,{children:(0,i.jsx)(n.code,{className:"language-bash",children:"$ ( source .my-env-file && starknet-devnet )\n"})}),"\n",(0,i.jsx)(n.h3,{id:"docker-1",children:"Docker"}),"\n",(0,i.jsxs)(n.p,{children:["To load environment variables from ",(0,i.jsx)(n.code,{children:".my-env-file"})," with Docker, remove the ",(0,i.jsx)(n.code,{children:"export"})," part in each line to have the file look like this:"]}),"\n",(0,i.jsx)(n.pre,{children:(0,i.jsx)(n.code,{children:"SEED=42\nACCOUNTS=3\n...\n"})}),"\n",(0,i.jsx)(n.p,{children:"Then run:"}),"\n",(0,i.jsx)(n.pre,{children:(0,i.jsx)(n.code,{children:"$ docker run --env-file .my-env-file shardlabs/starknet-devnet-rs\n"})})]})}function h(e={}){const{wrapper:n}={...(0,s.R)(),...e.components};return n?(0,i.jsx)(n,{...e,children:(0,i.jsx)(l,{...e})}):l(e)}},8453:(e,n,r)=>{r.d(n,{R:()=>o,x:()=>a});var i=r(6540);const s={},t=i.createContext(s);function o(e){const n=i.useContext(t);return i.useMemo((function(){return"function"==typeof e?e(n):{...n,...e}}),[n,e])}function a(e){let n;return n=e.disableParentContext?"function"==typeof e.components?e.components(s):e.components||s:o(e.components),i.createElement(t.Provider,{value:n},e.children)}}}]);
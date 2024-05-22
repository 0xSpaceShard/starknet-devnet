"use strict";(self.webpackChunkwebsite=self.webpackChunkwebsite||[]).push([[1914],{154:(e,n,s)=>{s.r(n),s.d(n,{assets:()=>a,contentTitle:()=>o,default:()=>h,frontMatter:()=>i,metadata:()=>d,toc:()=>l});var r=s(4848),t=s(8453);const i={sidebar_position:2.2},o="Run with Docker",d={id:"running/docker",title:"Run with Docker",description:"Devnet is available as a Docker image (Docker Hub link). To download the latest image, run:",source:"@site/versioned_docs/version-0.0.6/running/docker.md",sourceDirName:"running",slug:"/running/docker",permalink:"/starknet-devnet-rs/docs/running/docker",draft:!1,unlisted:!1,editUrl:"https://github.com/0xSpaceShard/starknet-devnet-rs/blob/master/website/versioned_docs/version-0.0.6/running/docker.md",tags:[],version:"0.0.6",sidebarPosition:2.2,frontMatter:{sidebar_position:2.2},sidebar:"docSidebar",previous:{title:"Install and run",permalink:"/starknet-devnet-rs/docs/running/install"},next:{title:"CLI options",permalink:"/starknet-devnet-rs/docs/running/cli"}},a={},l=[{value:"Docker image tags",id:"docker-image-tags",level:3},{value:"Container port publishing",id:"container-port-publishing",level:3},{value:"Linux",id:"linux",level:4},{value:"Mac and Windows",id:"mac-and-windows",level:4},{value:"Development note",id:"development-note",level:3}];function c(e){const n={a:"a",admonition:"admonition",code:"code",h1:"h1",h3:"h3",h4:"h4",li:"li",p:"p",pre:"pre",ul:"ul",...(0,t.R)(),...e.components};return(0,r.jsxs)(r.Fragment,{children:[(0,r.jsx)(n.h1,{id:"run-with-docker",children:"Run with Docker"}),"\n",(0,r.jsxs)(n.p,{children:["Devnet is available as a Docker image (",(0,r.jsx)(n.a,{href:"https://hub.docker.com/r/shardlabs/starknet-devnet-rs/",children:"Docker Hub link"}),"). To download the ",(0,r.jsx)(n.code,{children:"latest"})," image, run:"]}),"\n",(0,r.jsx)(n.pre,{children:(0,r.jsx)(n.code,{className:"language-text",children:"$ docker pull shardlabs/starknet-devnet-rs\n"})}),"\n",(0,r.jsx)(n.p,{children:"Supported platforms: linux/amd64 and linux/arm64 (also executable on darwin/arm64)."}),"\n",(0,r.jsxs)(n.p,{children:["Running a container is done like this (see ",(0,r.jsx)(n.a,{href:"#container-port-publishing",children:"port publishing"})," for more info):"]}),"\n",(0,r.jsx)(n.pre,{children:(0,r.jsx)(n.code,{className:"language-text",children:"$ docker run -p [HOST:]<PORT>:5050 shardlabs/starknet-devnet-rs [OPTIONS]\n"})}),"\n",(0,r.jsx)(n.h3,{id:"docker-image-tags",children:"Docker image tags"}),"\n",(0,r.jsx)(n.p,{children:"All of the versions published on crates.io for starknet-devnet are available as docker images, which can be used via:"}),"\n",(0,r.jsx)(n.pre,{children:(0,r.jsx)(n.code,{children:"$ docker pull shardlabs/starknet-devnet-rs:<CRATES_IO_VERSION>\n"})}),"\n",(0,r.jsx)(n.admonition,{type:"note",children:(0,r.jsxs)(n.p,{children:["The ",(0,r.jsx)(n.code,{children:"latest"})," docker image tag corresponds to the last published version on crates.io."]})}),"\n",(0,r.jsxs)(n.p,{children:["Commits to the ",(0,r.jsx)(n.code,{children:"main"})," branch of this repository are mostly available as images tagged with their commit hash (the full 40-lowercase-hex-digits SHA1 digest):"]}),"\n",(0,r.jsx)(n.pre,{children:(0,r.jsx)(n.code,{children:"$ docker pull shardlabs/starknet-devnet-rs:<COMMIT_HASH>\n"})}),"\n",(0,r.jsxs)(n.p,{children:["By appending the ",(0,r.jsx)(n.code,{children:"-seed0"})," suffix, you can use images which ",(0,r.jsx)(n.a,{href:"../predeployed",children:"predeploy funded accounts"})," with ",(0,r.jsx)(n.code,{children:"--seed 0"}),", thus always predeploying the same set of accounts:"]}),"\n",(0,r.jsx)(n.pre,{children:(0,r.jsx)(n.code,{children:"$ docker pull shardlabs/starknet-devnet-rs:<VERSION>-seed0\n$ docker pull shardlabs/starknet-devnet-rs:latest-seed0\n"})}),"\n",(0,r.jsx)(n.h3,{id:"container-port-publishing",children:"Container port publishing"}),"\n",(0,r.jsx)(n.h4,{id:"linux",children:"Linux"}),"\n",(0,r.jsxs)(n.p,{children:["If on a Linux host machine, you can use ",(0,r.jsx)(n.a,{href:"https://docs.docker.com/network/host/",children:(0,r.jsx)(n.code,{children:"--network host"})}),". This way, the port used internally by the container is also available on your host machine. The ",(0,r.jsx)(n.code,{children:"--port"})," option can be used (as well as other CLI options)."]}),"\n",(0,r.jsx)(n.pre,{children:(0,r.jsx)(n.code,{className:"language-text",children:"$ docker run --network host shardlabs/starknet-devnet-rs [--port <PORT>]\n"})}),"\n",(0,r.jsx)(n.h4,{id:"mac-and-windows",children:"Mac and Windows"}),"\n",(0,r.jsxs)(n.p,{children:["If not on Linux, you need to publish the container's internally used port to a desired ",(0,r.jsx)(n.code,{children:"<PORT>"})," on your host machine. The internal port is ",(0,r.jsx)(n.code,{children:"5050"})," by default (probably not your concern, but can be overridden with ",(0,r.jsx)(n.code,{children:"--port"}),")."]}),"\n",(0,r.jsx)(n.pre,{children:(0,r.jsx)(n.code,{className:"language-text",children:"$ docker run -p [HOST:]<PORT>:5050 shardlabs/starknet-devnet-rs\n"})}),"\n",(0,r.jsxs)(n.p,{children:["E.g. if you want to use your host machine's ",(0,r.jsx)(n.code,{children:"127.0.0.1:5050"}),", you need to run:"]}),"\n",(0,r.jsx)(n.pre,{children:(0,r.jsx)(n.code,{className:"language-text",children:"$ docker run -p 127.0.0.1:5050:5050 shardlabs/starknet-devnet-rs\n"})}),"\n",(0,r.jsxs)(n.p,{children:["You may ignore any address-related output logged on container startup (e.g. ",(0,r.jsx)(n.code,{children:"Starknet Devnet listening on 0.0.0.0:5050"}),"). What you will use is what you specified with the ",(0,r.jsx)(n.code,{children:"-p"})," argument."]}),"\n",(0,r.jsxs)(n.p,{children:["If you don't specify the ",(0,r.jsx)(n.code,{children:"HOST"})," part, the server will indeed be available on all of your host machine's addresses (localhost, local network IP, etc.), which may present a security issue if you don't want anyone from the local network to access your Devnet instance."]}),"\n",(0,r.jsx)(n.h3,{id:"development-note",children:"Development note"}),"\n",(0,r.jsx)(n.p,{children:"Due to internal needs, images with arch suffix are built and pushed to Docker Hub, but this is not mentioned in the user docs as users should NOT be needing it."}),"\n",(0,r.jsxs)(n.p,{children:["This is what happens under the hood on ",(0,r.jsx)(n.code,{children:"main"}),":"]}),"\n",(0,r.jsxs)(n.ul,{children:["\n",(0,r.jsxs)(n.li,{children:["build ",(0,r.jsx)(n.code,{children:"shardlabs/starknet-devnet-rs-<COMMIT_SHA1>-amd"})]}),"\n",(0,r.jsxs)(n.li,{children:["build ",(0,r.jsx)(n.code,{children:"shardlabs/starknet-devnet-rs-<COMMIT_SHA1>-arm"})]}),"\n",(0,r.jsxs)(n.li,{children:["create and push joint docker manifest called ",(0,r.jsx)(n.code,{children:"shardlabs/starknet-devnet-rs-<COMMIT_SHA1>"}),"\n",(0,r.jsxs)(n.ul,{children:["\n",(0,r.jsxs)(n.li,{children:["same for ",(0,r.jsx)(n.code,{children:"latest"})]}),"\n"]}),"\n"]}),"\n"]})]})}function h(e={}){const{wrapper:n}={...(0,t.R)(),...e.components};return n?(0,r.jsx)(n,{...e,children:(0,r.jsx)(c,{...e})}):c(e)}},8453:(e,n,s)=>{s.d(n,{R:()=>o,x:()=>d});var r=s(6540);const t={},i=r.createContext(t);function o(e){const n=r.useContext(i);return r.useMemo((function(){return"function"==typeof e?e(n):{...n,...e}}),[n,e])}function d(e){let n;return n=e.disableParentContext?"function"==typeof e.components?e.components(t):e.components||t:o(e.components),r.createElement(i.Provider,{value:n},e.children)}}}]);
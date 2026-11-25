#!/bin/sh
# Offline bundle used when npm install is unavailable (esbuild + React from a local tool folder). Normal builds use `npm run build` in web/.
set -e
cd "$(dirname "$0")/../web"
export TOOLS="${TOOLS:-/opt/npm-tools/node_modules}"
mkdir -p dist/assets
NODE_PATH="$TOOLS" node -e 'require(process.env.TOOLS+"/esbuild").build({entryPoints:["src/main.tsx"],bundle:true,minify:true,outfile:"dist/assets/app.js",jsx:"automatic",define:{"process.env.NODE_ENV":"\"production\""},nodePaths:[process.env.TOOLS]}).catch(()=>process.exit(1))'
sed 's#<script type="module" src="/src/main.tsx"></script>#<script type="module" src="/assets/app.js"></script>#; s#</head>#<link rel="stylesheet" href="/assets/app.css" /></head>#' index.html > dist/index.html

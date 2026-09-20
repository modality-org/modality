# modality-js

JavaScript packages for Modality, provided **as needed** for hosts that are not
Rust (Node, browsers, WASM).

Rust is canonical: the `modal` CLI and `modality-*` crates under `/rust`
implement the language, verifier, node, hub, and network. Add or keep JS here
when a JS host actually needs it. Prefer wrapping `modality-lang` WASM over
reimplementing the parser or model checker.

## Installation

```bash
pnpm i
```

## Running a Node

Configurations live in `packages/network-node/fixtures/configs`.

```bash
node packages/network-node/src/cmds/run.js --config packages/network-node/fixtures/configs/node1.json
```

You should see the node listening on the addresses in `node1.json`.

## Communication Between Nodes

### Ping

Start a second node and ping node 1 (use the `target` address from node 1):

```bash
node packages/network-node/src/cmds/ping.js --config packages/network-node/fixtures/configs/node2.json --target /ip4/127.0.0.1/tcp/10001/ws/p2p/12D3KooWPBRNBzgceXh7Z27wGoyYYz9ggwaYg2dWiwXXe8ieyFCN --times 10
```

### ReqRes

Valid paths include `/consensus/status`, `/consensus/sign_vertex`, and
`/consensus/submit_commits`.

```bash
node packages/network-node/src/cmds/request.js --config packages/network-node/fixtures/configs/node2.json --target /ip4/127.0.0.1/tcp/10001/ws/p2p/12D3KooWPBRNBzgceXh7Z27wGoyYYz9ggwaYg2dWiwXXe8ieyFCN --path "/consensus/status" --data "{\"hello\": \"world\"}"
```

## Start new datastore

```bash
node src/cmds/run.js --config ./fixtures/configs/node1.json --load_storage ./fixtures/datastores/devnet-static1.tgz --services scribe
```

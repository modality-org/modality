# modality-js

A modular framework for running and managing nodes with efficient communication between them.

> "All models are wrong but some are useful"
>
> - George Box

## Installation

To install the necessary dependencies, run:

```bash
pnpm i
```

## Running a Node

To run a node, you'll find the configurations in `packages/fixtures/network-node/fixtures/config`.

Execute the following command:

```bash
node packages/network-node/src/cmds/run.js --config packages/network-node/fixtures/configs/node1.json
```

You should now see in the terminal that you are listening on the addresses set in the `node1.json` config.

## Communication Between Nodes

### Ping

To start a second node and ping node 1 from node 2, use the `target` address from when you started `node1`:

```bash
node packages/network-node/src/cmds/ping.js --config packages/network-node/fixtures/configs/node2.json --target /ip4/127.0.0.1/tcp/10001/ws/p2p/12D3KooWPBRNBzgceXh7Z27wGoyYYz9ggwaYg2dWiwXXe8ieyFCN --times 10
```

### ReqRes

You can communicate directly between nodes and pass data by running the ReqRes command and specifying a `path` and `data`.

#### Valid Paths

- `/consensus/status`
- `/consensus/sign_vertex`
- `/consensus/submit_commits`

Example command:

```bash
node packages/network-node/src/cmds/request.js --config packages/network-node/fixtures/configs/node2.json --target /ip4/127.0.0.1/tcp/10001/ws/p2p/12D3KooWPBRNBzgceXh7Z27wGoyYYz9ggwaYg2dWiwXXe8ieyFCN --path "/consensus/status" --data "{\"hello\": \"world\"}"
```

Feel free to reach out for any issues or contributions!

## Start new datastore

node src/cmds/run.js --config ./fixtures/configs/node1.json --load_storage ./fixtures/datastores/devnet-static1.tgz --services scribe

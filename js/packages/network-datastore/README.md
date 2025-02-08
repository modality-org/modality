# Network Datastore

This is a RocksDB based datastore for Modality Network data.

## Models

* `Block`: a unit of 

## Schema

### `/blocks/`

This holds block related data used by full nodes. An archive of this subpath suffices for standing up a full node.

* `/blocks/round/:round_id/block/:peer_id`

### `/block_headers/`

This holds partial records of blocks, assumed correct by lite nodes. An archive of this subpath suffices for standing up a lite nodes.

* `/block_headers/round/:round_id/block/:peer_id`

### `/block_metas/`

This holds consensus data derived post-communication. For example, the precise sequencing of blocks.

* `/block_metas/round/:round_id/block/:peer_id`

### `/block_messages/`

This holds consensus messages received by a node, but not yet consumed. 

* `/block_messages/round/:round_id/type/:type/peer/:peer_id`

### `/transactions/`

This holds transactions waiting to be sequenced by a node 

* `/transactions/:timestamp/:contract_id/:commit_id`

### `/alternates/`

This holds alternate, possibly equivocating, block data, kept by a node during a consensus fork.

* `/alternate/block/round/:round_id/block/:peer_id/hash/:hash`
* `/alternate/block_headers/round/:round_id/block/:peer_id/hash/:hash`

### `/node_status/`

This holds records related to the status of local node.


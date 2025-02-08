# network-consensus

## Overview

The Modality Network orders and validates commits within a global decentralized network of verifiable contracts.

### Commit Ordering

Nodes called Scribes work together to record commits in parallel during periods of time called Rounds. Each Scribe records the commits sent to them on a Block. When a Scribe completes a Block, the Scribe gossips that Block to the other Scribes, who are then responsible for sending an Ack (acknowledgement) back. The source Scribe collects a sufficient number of the Acks to produce a Cert (certificate). The source Scribe gossips the Cert to the other Scribes, and then moves on to work on a new Block for the next Round. (See the [Narwhal communication](https://arxiv.org/pdf/2105.11827) for reliable dissemination.)

Processes called Sequencers work under Scribes. Sequencers are responsible for ordering the Blocks written and collected by Scribes. Different binding method exists and the Scribes are responsible for agreeing on the method ahead of time. In general, a binding method uses a source of common randomness to select the first Block from a Round (the Round Anchor Block). Then, all the not yet bound Blocks linked by Acks starting at the Round Anchor Block are causally ordered. Causal ordering means that a Block is always bound before any of the Blocks that acknowledge it. At the end of each binding, a Section is produced that finalizes the total ordering of the Blocks. Amazingly, the Acks sent between Scribes suffice for the Sequencers to do their work, and Sequencers do not need to send additional messages to achieve consensus on the total ordering of Blocks and their commits. (See the [DAG Rider consensus](https://arxiv.org/abs/2102.08325) for more details on ordering consensus.)

## Commit Validation

The secondary purpose of Modality network consensus is to logically validate commits. Logical validation is optional unless a commit affects the validity of another contract. When a commit affects the validity of another contract, the source contract requires validation through its history up until the affecting commits.

Nodes called Validators work together to validate commits.

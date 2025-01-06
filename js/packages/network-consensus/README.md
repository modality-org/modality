# network-consensus

## Overview

The primary purpose of Modality network consensus is to canonically order commits.

Nodes called Scribes work together to record commits in parallel during periods of time called Rounds. Each Scribe records the commits sent to them on a Page. When a Scribe completes a Page, the Scribe gossips that Page to the other Scribes, who are then responsible for sending an Ack (acknowledgement) back. The source Scribe collects a sufficient number of the Acks to produce a Cert (certificate). The source Scribe gossips the Cert to the other Scribes, and then moves on to work on a new Page for the next Round. (See the [Narwhal communication](https://arxiv.org/pdf/2105.11827) for reliable dissemination.)

Processes called Sequencers work under Scribes. Sequencers are responsible for ordering the Pages written and collected by Scribes. Different binding method exists and the Scribes are responsible for agreeing on one ahead of time. In general, a binding method uses a source of common randomness to select the first Page from a Round (the Round Selected Page). Then, all the not yet bound Pages linked by Acks starting at the Round Selected Page are causally ordered. Causal ordering means that a Page is always bound before any of the Pages that acknowledge it. At the end of each binding, a Section is produced that finalizes the total ordering of the Pages. Amazingly, the Acks sent between Scribes suffice for the Sequencers to do their work, and Sequencers do not need to send additional messages to achieve consensus on the total ordering of Pages and their commits. (See the [DAG Rider consensus](https://arxiv.org/abs/2102.08325) for more details on ordering consensus.)

The secondary purpose of Modality network consensus is to logically validate commits. Logical validation is optional unless a commit affects the validity of another contract. When a commit affects the validity of another contract, the source contract requires validation through its history up until the affecting commits.

Nodes called Validators work together to validate commits.

import { jest, expect, describe, test, it } from "@jest/globals";

import createTestNode from "../../createTestNode";
import node1_config from "../../../fixtures/configs/node1.json";
import node2_config from "../../../fixtures/configs/node2.json";

import { addSequencerEventListeners } from "../../../src/gossip/index.js";
import { setupNode } from "../../../src/lib/setupNode.js";
import Page from '@modality-dev/network-datastore/data/Page';
import Round from '@modality-dev/network-datastore/data/Round';
import Keypair from "@modality-dev/utils/Keypair";

describe("gossip /consensus/scribes/page_draft", () => {
  it.skip("should work", async () => {
    const node1 = await createTestNode(node1_config);
    await addSequencerEventListeners(node1);
    await setupNode(node1, node1_config);

    const node2 = await createTestNode(node2_config);
    await addSequencerEventListeners(node2);
    await setupNode(node2, node2_config);

    const round = Round.from({
      round: 1,
      scribes: [node1_config.keypair.id, node2_config.keypair.id],
    });
    await round.save({datastore: node1.storage.datastore});
    await round.save({datastore: node2.storage.datastore});

    const mockListener = jest.fn();
    node2.services.pubsub.addEventListener("message", mockListener);

    const page = Page.from({
      round: 1,
      scribe: node1_config.keypair.id,
      last_round_certs: [],
      events: [],
    });
    const node1_keypair = await Keypair.fromJSON(node1_config.keypair);
    await page.generateSig(node1_keypair);
    const json_data = page.toDraftJSONObject();
    await node1.services.pubsub.publish(
      "/consensus/scribes/page_draft",
      new TextEncoder().encode(JSON.stringify(json_data))
    );

    expect(mockListener).toHaveBeenCalled();
  });
});

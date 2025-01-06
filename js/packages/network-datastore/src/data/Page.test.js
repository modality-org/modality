import { jest, expect, describe, test, it } from "@jest/globals";

import Page from "./Page.js";
import NetworkDatastore from "../NetworkDatastore.js";

import Keypair from "@modality-dev/utils/Keypair";

describe("Page", () => {
  test("should work", async () => {
    const datastore = await NetworkDatastore.createInMemory();

    const node1_keypair = await Keypair.generate();
    const node1_peerid = await node1_keypair.asPublicAddress();

    const node2_keypair = await Keypair.generate();
    const node2_peerid = await node1_keypair.asPublicAddress();

    let b1 = Page.from({ scribe: node1_peerid, round: 1, events: [] });
    await b1.addEvent({ data: "data1" });
    await b1.addEvent({ data: "data2" });
    expect(b1.events.length).toBe(2);
    let sig1 = await b1.generateSig(node1_keypair);
    let result = await b1.validateSig();
    expect(result).toBe(true);
    let b1empty = Page.from({ scribe: node1_peerid, round: 1, events: [] });
    let sig1empty = await b1empty.generateSig(node1_keypair);
    expect(sig1).not.toBe(sig1empty);

    // ack self
    let ack1 = await b1.generateAck(node1_keypair);
    await b1.addAck(ack1);
    result = await b1.countValidAcks();
    expect(result).toBe(1);

    // other acks
    let ack2 = await b1.generateAck(node2_keypair);
    await b1.addAck(ack2);
    expect(b1.acks[ack2.acker]).toBe(ack2);
    result = await b1.validateAcks();
    expect(result).toBe(true);
    result = await b1.countValidAcks();
    expect(result).toBe(2);

    await b1.generateCert(node1_keypair);
    expect(b1.cert).not.toBe(null);
    result = await b1.validateCert({ acks_needed: 2 });
    expect(result).toBe(true);
    await b1.save({ datastore });

    result = b1.getId();
    expect(result).toBe(`/consensus/round/1/scribe/${node1_peerid}`);
    const b1r = await Page.findOne({
      datastore,
      round: 1,
      scribe: node1_peerid,
    });
    expect(b1r.cert).toBe(b1.cert);
  });
});

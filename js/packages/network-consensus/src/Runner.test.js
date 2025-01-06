import { jest, expect, describe, test, it } from "@jest/globals";

import NetworkDatastoreBuilder from "@modality-dev/network-datastore/NetworkDatastoreBuilder";
import Devnet from "@modality-dev/network-configs/Devnet";
import Page from "@modality-dev/network-datastore/data/Page";

import Runner from "./Runner";
import StaticAuthority from "./sequencing/StaticAuthority";

describe("Runner", () => {
  test("event handling", async () => {
    const NODE_COUNT = 3;

    // const election = RoundRobin.create();

    let page, ack, round;

    // setup
    const scribes = await Devnet.getPeerids(NODE_COUNT);
    const scribe_keypairs = await Devnet.getKeypairsDict(NODE_COUNT);
    const sequencing = await StaticAuthority.create({scribes});

    const ds_builder = await NetworkDatastoreBuilder.createInMemory();
    ds_builder.scribes = [...scribes];
    ds_builder.scribe_keypairs = scribe_keypairs;
    ds_builder.datastore.setCurrentRound(1);
    await ds_builder.addFullyConnectedRound();

    const datastores = [
      await ds_builder.datastore.cloneToMemory(),
      await ds_builder.datastore.cloneToMemory(),
      await ds_builder.datastore.cloneToMemory(),
    ];

    const runner1 = Runner.create({
      datastore: datastores[0],
      peerid: scribes[0],
      keypair: scribe_keypairs[scribes[0]],
      communication_enabled: true,
      sequencing,
    });

    const runner2 = Runner.create({
      datastore: datastores[1],
      peerid: scribes[1],
      keypair: scribe_keypairs[scribes[1]],
      sequencing,
    });

    const runner3 = Runner.create({
      datastore: datastores[2],
      peerid: scribes[2],
      keypair: scribe_keypairs[scribes[2]],
      sequencing,
    });

    // round 2 from perspective of scribe 1
    round = 2;
    page = Page.from({
      round,
      scribe: scribes[0],
      last_round_certs: await runner1.datastore.getTimelyCertSigsAtRound(round - 1),
      events: [],
    });
    await page.generateSig(scribe_keypairs[scribes[0]]);
    await page.save({ datastore: runner1.datastore });
    ack = await runner1.onReceiveDraftPage(page);
    await runner1.onReceivePageAck(ack);

    ack = await runner2.onReceiveDraftPage(page);
    await runner1.onReceivePageAck(ack);

    ack = await runner3.onReceiveDraftPage(page);
    await runner1.onReceivePageAck(ack);

    await page.reload({ datastore: runner1.datastore });
    await page.generateCert(scribe_keypairs[scribes[0]]);
    expect(page.cert).not.toBeNull();
    expect(Object.keys(page.acks).length).toBe(3);
    expect(await page.validateCert({ acks_needed: 3 })).toBe(true);

    let cert_page = await runner2.onReceiveCertifiedPage(
      await page.toJSONObject()
    );
    expect(cert_page).not.toBe(null);
    cert_page = await runner2.onReceiveCertifiedPage({
      ...(await page.toJSONObject()),
      cert: null,
    });
    expect(cert_page).toBeNull();
  });
});
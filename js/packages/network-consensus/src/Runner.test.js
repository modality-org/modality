import { jest, expect, describe, test, it } from "@jest/globals";

import NetworkDatastoreBuilder from "@modality-dev/network-datastore/NetworkDatastoreBuilder";
import Devnet from "@modality-dev/network-configs/Devnet";
import Block from "@modality-dev/network-datastore/data/Block";

import Runner from "./Runner";
import StaticAuthority from "./sequencing/StaticAuthority";

describe("Runner", () => {
  test("event handling", async () => {
    const NODE_COUNT = 3;

    // const election = RoundRobin.create();

    let block, ack, round;

    // setup
    const scribes = await Devnet.getPeerids(NODE_COUNT);
    const scribe_keypairs = await Devnet.getKeypairsDict(NODE_COUNT);
    const sequencing = await StaticAuthority.create({ scribes });

    const ds_builder = await NetworkDatastoreBuilder.createInMemory();
    ds_builder.scribes = [...scribes];
    ds_builder.scribe_keypairs = scribe_keypairs;
    ds_builder.datastore.setCurrentRound(0);
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
    round = 1;
    const prev_round_certs = await runner1.datastore.getTimelyCertSigsAtRound(
      round - 1
    );
    block = Block.from({
      round_id: round,
      peer_id: scribes[0],
      prev_round_certs,
      events: [],
    });
    await block.generateSig(scribe_keypairs[scribes[0]]);
    await block.save({ datastore: runner1.datastore });
    ack = await runner1.onReceiveBlockDraft(block);
    await runner1.onReceiveBlockAck(ack);

    ack = await runner2.onReceiveBlockDraft(block);
    await runner1.onReceiveBlockAck(ack);

    ack = await runner3.onReceiveBlockDraft(block);
    await runner1.onReceiveBlockAck(ack);

    await block.reload({ datastore: runner1.datastore });
    await block.generateCert(scribe_keypairs[scribes[0]]);
    expect(block.cert).not.toBeNull();
    expect(Object.keys(block.acks).length).toBe(3);
    expect(await block.validateCert({ acks_needed: 3 })).toBe(true);

    let cert_block = await runner2.onReceiveBlockCert(block.toJSONObject());
    expect(cert_block).not.toBe(null);
    cert_block = await runner2.onReceiveBlockCert({
      ...block.toJSONObject(),
      cert: null,
    });
    expect(cert_block).toBeNull();
  });
});

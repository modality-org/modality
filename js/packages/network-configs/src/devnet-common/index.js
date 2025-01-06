import { readFile } from "fs/promises";

import Keypair from "@modality-dev/utils/Keypair";

const keypairs = JSON.parse(
  await readFile(new URL("./keypairs.json", import.meta.url))
);

export async function getKeypairs(count = null) {
  if (!count) {
    count = Object.keys(keypairs).length;
  }
  if (count > Object.keys(keypairs).length) {
    throw new Error("not enough common IDs");
  }
  const slice = Object.values(keypairs).slice(0, count);
  const r = [];
  for (const kp of slice) {
    r.push(await Keypair.fromJSON(kp));
  }
  return r;
}

export async function getKeypairByIndex(index) {
  if (index > Object.keys(keypairs).length) {
    throw new Error("not enough common IDs");
  }
  return await Keypair.fromJSON(Object.values(keypairs)[index]);
}

export async function getKeypairsDict(count = null) {
  if (!count) {
    count = Object.keys(keypairs).length;
  }
  if (count > Object.keys(keypairs).length) {
    throw new Error("not enough common IDs");
  }
  const slice = Object.values(keypairs).slice(0, count);
  const r = {};
  for (const kp of slice) {
    r[kp.id] = await Keypair.fromJSON(kp);
  }
  return r;
}

export async function getPeerids(count = null) {
  if (!count) {
    count = Object.keys(keypairs).length;
  }
  if (count > Object.keys(keypairs).length) {
    throw new Error("not enough common IDs");
  }
  const slice = Object.values(keypairs).slice(0, count);
  const r = [];
  for (const kp of slice) {
    r.push(kp.id);
  }
  return r;
}

export function peeridOf(index) {
  return Object.values(keypairs)[index]?.id;
}

export function indexOf(peerid) {
  let index = 0
  for (const keypair of Object.entries(keypairs)) {
    if (keypair[1].id === peerid) {
      return index;
    }
    index++;
  }
}

export async function getKeypairFor(id) {
  return await Keypair.fromJSON(keypairs[id]);
}

export default {
  name: "devnet-common",
  keypairs,
  getPeerids,
  getKeypairs,
  getKeypairByIndex,
  getKeypairFor,
  getKeypairsDict,
  peeridOf,
  indexOf
};

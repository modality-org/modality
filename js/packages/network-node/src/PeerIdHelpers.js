import { readFileSync } from "fs";
import { toString as uint8ArrayToString } from "uint8arrays/to-string";
import { createEd25519PeerId } from "@libp2p/peer-id-factory";
import { createFromJSON as _createFromJSON } from "@libp2p/peer-id-factory";

export function createFromJSON(obj) {
  return _createFromJSON({
    id: obj.id,
    pubKey: obj.public_key || obj.publicKey || obj.pubKey,
    privKey: obj.private_key || obj.privateKey || obj.privKey,
  });
}

export function createFromJSONString(str) {
  const obj = JSON.parse(str);
  return _createFromJSON({
    id: obj.id,
    publicKey: obj.public_key || obj.publicKey,
    privateKey: obj.private_key || obj.privateKey,
  });
}

export async function create() {
  return createEd25519PeerId();
}

export function exportToJSON(peerId, excludePrivateKey = false) {
  return {
    id: peerId.toString(),
    public_key: uint8ArrayToString(peerId.publicKey, "base64pad"),
    private_key:
      excludePrivateKey === true || peerId.privateKey == null
        ? undefined
        : uint8ArrayToString(peerId.privateKey, "base64pad"),
  };
}

export function exportToJSONString(peerId, excludePrivateKey = false) {
  return JSON.stringify(exportToJSON(peerId, excludePrivateKey));
}

export function createFromJSONFile(path) {
  const json = readFileSync(path, "utf-8");
  return createFromJSONString(json);
}

export default {
  exportToJSON,
  exportToJSONString,
  create,
  createFromJSONString,
  createFromJSONFile,
  createFromJSON,
};

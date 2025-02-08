import { writeFileSync, readFileSync } from "fs";
import {
  generateKeyPair,
  privateKeyFromRaw,
  publicKeyFromRaw,
} from "@libp2p/crypto/keys";
import { toString as uint8ArrayToString } from "uint8arrays/to-string";
import { fromString as uint8ArrayFromString } from "uint8arrays/from-string";
import { base58btc } from "multiformats/bases/base58";
import { identity } from "multiformats/hashes/identity";
import * as Digest from "multiformats/hashes/digest";
import { peerIdFromString } from "@libp2p/peer-id";
import JSONStringifyDeterministic from "json-stringify-deterministic";
import SSHPem from "./SSHPem.js";
import * as EncryptedText from "./EncryptedText.js";

export default class Keypair {
  constructor(key) {
    this.key = key;
    return this;
  }

  static async generate(type = "Ed25519") {
    const key = await generateKeyPair(type);
    return new Keypair(key);
  }

  async asPeerId() {
    return peerIdFromString(await this.publicKeyAsBase58Identity());
  }

  // SSH keys

  async asSSHPrivatePem(comment = "") {
    const pem = new SSHPem();
    pem.key_type = "ed25519";
    pem.public_key = this.key.publicKey.toMultihash().bytes.subarray(4);
    pem.private_key = this.key.raw.subarray(4);
    pem.comment = comment;
    return pem.toSSHPrivatePemString();
  }

  asSSHDotPub(comment = "") {
    const pem = new SSHPem();
    pem.key_type = "ed25519";
    pem.public_key = this.key.publicKey.toMultihash().bytes.subarray(4);
    pem.comment = comment;
    return pem.toSSHDotPubString();
  }

  static fromSSHDotPub(public_key_str, type = "ed25519") {
    const inner_pubkey_str = public_key_str.split(" ")[1];
    const inner_pubkey_uint8a = uint8ArrayFromString(
      inner_pubkey_str,
      "base64"
    );
    const pubkey_uint8a = new Uint8Array([
      8,
      1,
      18,
      32,
      ...inner_pubkey_uint8a.subarray(19),
    ]);
    const public_key_id = Keypair.uint8ArrayAsBase58Identity(pubkey_uint8a);
    return this.fromPublicMultiaddress(`/${type}-pub/${public_key_id}`);
  }

  // base58identity is used for addresses

  static uint8ArrayAsBase58Identity(bytes) {
    const encoding = identity.digest(bytes);
    return base58btc.encode(encoding.digest).substring(1);
  }

  static keyAsBase58Identity(key) {
    return this.uint8ArrayAsBase58Identity(key.toMultihash().bytes);
  }

  async publicKeyAsBase58Identity() {
    return this.constructor.keyAsBase58Identity(this.key.publicKey);
  }

  async asPublicKeyId() {
    return await this.publicKeyAsBase58Identity();
  }

  async asPublicAddress() {
    return await this.publicKeyAsBase58Identity();
  }

  // base64pad is used for storage

  static uint8ArrayAsBase64Pad(bytes) {
    return uint8ArrayToString(bytes, "base64pad");
  }

  static keyAsBase64Pad(raw) {
    const bytes = new Uint8Array([8, 1, 18, 32, ...raw]);
    return this.uint8ArrayAsBase64Pad(bytes);
  }

  async publicKeyAsBase64Pad() {
    return this.constructor.keyAsBase64Pad(
      this.key.publicKey.toMultihash().bytes
    );
  }

  async privateKeyAsBase64Pad() {
    return this.constructor.keyAsBase64Pad(this.key.raw);
  }

  // multiaddr is used in modality

  async publicKeyToMultiaddrString() {
    const public_key_id = await this.publicKeyAsBase58Identity();
    const type = "ed25519";
    return `/${type}-pub/${public_key_id}`;
  }

  async asPublicMultiaddress() {
    return await this.publicKeyToMultiaddrString();
  }

  static fromPublicKey(public_key_id, type = "ed25519") {
    return this.fromPublicMultiaddress(`/${type}-pub/${public_key_id}`);
  }

  static fromPublicMultiaddress(multiaddress) {
    // TODO for real
    const m = multiaddress.match(/^(.+)-pub\/(.+)$/);
    if (!m) {
      return;
    }
    const type = m[1];
    const public_key_id = m[2];
    const multihash = Digest.decode(base58btc.decode(`z${public_key_id}`));
    const key = new Keypair({
      publicKey: publicKeyFromRaw(multihash.digest.subarray(4)),
    });
    return key;
  }

  // json

  static async fromJSON({ id, public_key, private_key }) {
    if (private_key) {
      const raw = uint8ArrayFromString(private_key, "base64pad").subarray(4);
      const key = privateKeyFromRaw(raw);
      return new Keypair(key);
    } else if (public_key) {
      const raw = uint8ArrayFromString(public_key, "base64pad").subarray(4);
      const key = publicKeyFromRaw(raw);
      return new Keypair({ publicKey: key });
    }
  }

  static async fromJSONString(str) {
    return await this.fromJSON(JSON.parse(str));
  }

  static async fromJSONFile(fp) {
    return await this.fromJSONString(readFileSync(fp));
  }

  async asPublicJSON() {
    const id = await this.publicKeyAsBase58Identity();
    const public_key = await this.publicKeyAsBase64Pad();
    return {
      id,
      public_key,
    };
  }

  async asPublicJSONString() {
    const json = await this.toPublicJSON();
    return JSON.stringify(json);
  }

  async asPublicJSONFile(path) {
    const json_string = await this.toPublicJSONString();
    return writeFileSync(path, json_string, "utf-8");
  }

  async asJSON() {
    const id = await this.publicKeyAsBase58Identity();
    const public_key = await this.publicKeyAsBase64Pad();
    const private_key = await this.privateKeyAsBase64Pad();
    return {
      id,
      public_key,
      private_key,
    };
  }

  async asJSONString() {
    const json = await this.asJSON();
    return JSON.stringify(json);
  }

  async asJSONFile(path) {
    const json_string = await this.asJSONString();
    return writeFileSync(path, json_string, "utf-8");
  }

  static async fromEncryptedJSON(
    { id, public_key, encrypted_private_key },
    password
  ) {
    const private_key = await EncryptedText.decrypt(
      encrypted_private_key,
      password
    );
    return this.fromJSON({ id, public_key, private_key });
  }

  static async fromEncryptedJSONString(str, password) {
    return await this.fromEncryptedJSON(JSON.parse(str), password);
  }

  static async fromEncryptedJSONFile(fp, password) {
    return await this.fromEncryptedJSONString(readFileSync(fp), password);
  }

  async asEncryptedJSON(password) {
    const id = await this.publicKeyAsBase58Identity();
    const public_key = await this.publicKeyAsBase64Pad();
    const private_key = await this.privateKeyAsBase64Pad();
    const encrypted_private_key = await EncryptedText.encrypt(
      private_key,
      password
    );
    return {
      id,
      public_key,
      encrypted_private_key,
    };
  }

  async asEncryptedJSONString(password) {
    const json = await this.asEncryptedJSON(password);
    return JSON.stringify(json);
  }

  async asEncryptedJSONFile(path, password) {
    const json_string = await this.asEncryptedJSONString(password);
    return writeFileSync(path, json_string, "utf-8");
  }

  // signing

  async signBytes(bytes) {
    return this.key.sign(bytes);
  }

  async signString(str) {
    const bytes = uint8ArrayFromString(str);
    return await this.signBytes(bytes);
  }

  async signStringAsBase64Pad(str) {
    const signature_in_bytes = await this.signString(str);
    return uint8ArrayToString(signature_in_bytes, "base64pad");
  }

  async signFile() {}

  async signJSON(json) {
    const str = JSONStringifyDeterministic(json);
    return await this.signStringAsBase64Pad(str);
  }

  async signJSONElement(json, name, suffix = ".signature") {
    const signature = await this.signJSON(json[name]);
    return {
      ...json,
      [`${name}${suffix}`]: signature,
    };
  }

  async signJSONAsKey(json, key = "signature") {
    const signature = await this.signJSON(json);
    return {
      ...json,
      [key]: signature,
    };
  }

  // verify signatures

  async verifySignatureForBytes(signature, bytes) {
    const signature_bytes = uint8ArrayFromString(signature, "base64pad");
    return await this.key.publicKey.verify(bytes, signature_bytes);
  }

  async verifySignatureForString(signature, str) {
    const bytes = uint8ArrayFromString(str);
    try {
      return await this.verifySignatureForBytes(signature, bytes);
    } catch (e) {
      return false;
    }
  }

  async verifyJSON(signature, json) {
    const str = JSONStringifyDeterministic(json);
    return await this.verifySignatureForString(signature, str);
  }

  async verifySignatureForJson(signature, json) {
    const str = JSONStringifyDeterministic(json);
    return await this.verifySignatureForString(signature, str);
  }

  async verifySignaturesInJson(json, suffix = ".signature") {
    const suffix_for_regex = suffix
      .replace(/[|\\{}()[\]^$+*?.]/g, "\\$&")
      .replace(/-/g, "\\x2d");
    const suffix_regex = `(.+)${suffix_for_regex}$`;
    for (const name in json) {
      const m = name.match(suffix_regex);
      if (m) {
        const str = JSONStringifyDeterministic(json[m[1]]);
        const sig = json[name];
        const r = await this.verifySignatureForString(sig, str);
        if (!r) {
          return false;
        }
      }
    }
    return true;
  }
}

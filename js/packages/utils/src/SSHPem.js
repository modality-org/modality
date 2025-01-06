import { toString as uint8ArrayToString } from "uint8arrays/to-string";
import { fromString as uint8ArrayFromString } from "uint8arrays/from-string";

function declaredWidthBytes(bytes, declarationWidth = 4) {
  const hex_size = bytes.length.toString(16);
  const hex = hex_size
    .padStart(declarationWidth * 2, "0")
    .match(/(.{1,2})/g)
    .map((i) => parseInt(i, 16));
  const dec_bytes = new Uint8Array(hex);
  return new Uint8Array([...dec_bytes, ...bytes]);
}

function declaredWidthString(str, declarationWidth = 4) {
  const bytes = uint8ArrayFromString(str, "ascii");
  return declaredWidthBytes(bytes, declarationWidth);
}

function sumMSBArray(arr, base) {
  let r = 0;
  let i = 0;
  for (const item of [...arr].reverse()) {
    r = r + item * base ** i;
    i++;
  }
  return r;
}

function unshiftBytesFromBytes(bytes_box, width = 4) {
  const { bytes } = bytes_box;
  const r = bytes.subarray(0, width);
  bytes_box.bytes = bytes.subarray(width);
  return r;
}

function getDeclaredWidthDetailsFromBytes(bytes, declarationWidth = 4) {
  const length = sumMSBArray(bytes.subarray(0, declarationWidth), 16);
  return {
    full_length: declarationWidth + length,
    bytes: bytes.subarray(declarationWidth, declarationWidth + length),
    length,
  };
}

function unshiftDeclaredWidthBytesFromBytes(bytes_box, declarationWidth = 4) {
  const { bytes } = bytes_box;
  const { bytes: content_bytes, full_length } =
    getDeclaredWidthDetailsFromBytes(bytes, declarationWidth);
  bytes_box.bytes = bytes.subarray(full_length);
  return content_bytes;
}

function unshiftDeclaredWidthStringFromBytes(bytes_box, declarationWidth = 4) {
  const { bytes } = bytes_box;
  const { bytes: content_bytes, full_length } =
    getDeclaredWidthDetailsFromBytes(bytes, declarationWidth);
  const content = uint8ArrayToString(content_bytes);
  bytes_box.bytes = bytes.subarray(full_length);
  return content;
}

function unshiftFixedWidthIntFromBytes(bytes_box, width = 4) {
  const { bytes } = bytes_box;
  const content = sumMSBArray(bytes.subarray(0, width), 16);
  bytes_box.bytes = bytes.subarray(width);
  return content;
}

function pushBytesToBytes(bytes_box, bytes) {
  bytes_box.bytes = new Uint8Array([...bytes_box.bytes, ...bytes]);
  return bytes_box.bytes;
}

function pushStringToBytes(bytes_box, str) {
  bytes_box.bytes = new Uint8Array([
    ...bytes_box.bytes,
    ...uint8ArrayFromString(str),
  ]);
  return bytes_box.bytes;
}

function pushDeclaredWidthBytesToBytes(bytes_box, str, declarationWidth = 4) {
  bytes_box.bytes = new Uint8Array([
    ...bytes_box.bytes,
    ...declaredWidthBytes(str, declarationWidth),
  ]);
  return bytes_box.bytes;
}

function pushDeclaredWidthStringToBytes(bytes_box, str, declarationWidth = 4) {
  bytes_box.bytes = new Uint8Array([
    ...bytes_box.bytes,
    ...declaredWidthString(str, declarationWidth),
  ]);
  return bytes_box.bytes;
}

function pushFixedWidthIntToBytes(bytes_box, int, width = 4) {
  const new_bytes = new Number(int)
    .toString(16)
    .padStart(width * 2, 0)
    .match(/(.{1,2})/g)
    .map((i) => parseInt(i, 16));
  bytes_box.bytes = new Uint8Array([...bytes_box.bytes, ...new_bytes]);
  return bytes_box.bytes;
}

export default class SSHPem {
  constructor() {
    this.cipher_name = "none";
    this.kdfname = "none";
    this.kdfopts = "";
    this.num_of_keys = 1;
    this.key_type = "ed25519";
    this.public_key = null;
    this.private_key = null;
    this.comment = "";
    this.details = {};
    return this;
  }

  toSSHPrivatePemString() {
    const bytes = [];
    const bytes_box = { bytes };

    pushStringToBytes(bytes_box, "openssh-key-v1");
    pushBytesToBytes(bytes_box, new Uint8Array([0]));

    pushDeclaredWidthStringToBytes(bytes_box, this.cipher_name);
    pushDeclaredWidthStringToBytes(bytes_box, this.kdfname);
    pushDeclaredWidthStringToBytes(bytes_box, this.kdfopts);
    pushFixedWidthIntToBytes(bytes_box, this.num_of_keys);

    const pubkey_section_length =
      4 +
      uint8ArrayFromString(`ssh-${this.key_type}`).length +
      4 +
      this.public_key.length;
    pushFixedWidthIntToBytes(bytes_box, pubkey_section_length);
    pushDeclaredWidthStringToBytes(bytes_box, `ssh-${this.key_type}`);
    pushDeclaredWidthBytesToBytes(bytes_box, this.public_key);

    const checkints = this.details.checkints || new Uint8Array([1, 2, 3, 4]);
    const pk_section_length_sans_padding =
      checkints.length +
      checkints.length +
      4 +
      uint8ArrayFromString(`ssh-${this.key_type}`, "ascii").length +
      4 +
      this.public_key.length +
      4 +
      this.private_key.length +
      4 +
      uint8ArrayFromString(this.comment, "ascii").length;
    const BLOCKSIZE = 8;
    const padding_length =
      BLOCKSIZE - (pk_section_length_sans_padding % BLOCKSIZE || BLOCKSIZE);
    const pk_section_length = pk_section_length_sans_padding + padding_length;
    pushFixedWidthIntToBytes(bytes_box, pk_section_length);
    pushBytesToBytes(bytes_box, checkints);
    pushBytesToBytes(bytes_box, checkints);
    pushDeclaredWidthStringToBytes(bytes_box, `ssh-${this.key_type}`);
    pushDeclaredWidthBytesToBytes(bytes_box, this.public_key);
    pushDeclaredWidthBytesToBytes(bytes_box, this.private_key);
    pushDeclaredWidthStringToBytes(bytes_box, this.comment);

    const final_bytes = new Uint8Array(
      Array.from({ length: 15 }, (_, i) => i + 1)
    ).subarray(0, padding_length);
    pushBytesToBytes(bytes_box, final_bytes);

    return `-----BEGIN OPENSSH PRIVATE KEY-----
${uint8ArrayToString(bytes_box.bytes, "base64pad")
  .match(/.{1,70}/g)
  .join("\n")}
-----END OPENSSH PRIVATE KEY-----`;
  }

  static fromSSHPrivatePemString(pem_str) {
    const pem = new SSHPem();
    if (!pem_str.match(`-----BEGIN OPENSSH PRIVATE KEY-----`)) {
      throw new Error("pem not supported");
    }
    const pem_hex = pem_str
      .split("\n")
      .filter((line) => !line.match(/^---/))
      .join("");
    const pem_bytes = uint8ArrayFromString(pem_hex, "base64");

    let wip_bytes = new Uint8Array([...pem_bytes]);
    const preamble = uint8ArrayToString(wip_bytes.subarray(0, 14));
    if (preamble !== "openssh-key-v1") {
      throw new Error("pem preamble not openssh-key-v1");
    }
    wip_bytes = wip_bytes.subarray(15);
    const wip_bytes_box = { bytes: wip_bytes };

    const r = {};
    r.cipher_name = unshiftDeclaredWidthStringFromBytes(wip_bytes_box);
    r.kdfname = unshiftDeclaredWidthStringFromBytes(wip_bytes_box);
    r.kdfopts = unshiftDeclaredWidthStringFromBytes(wip_bytes_box);
    r.num_of_keys = unshiftFixedWidthIntFromBytes(wip_bytes_box);
    r.pubkey_section_length = unshiftFixedWidthIntFromBytes(wip_bytes_box);
    r.key_type = unshiftDeclaredWidthStringFromBytes(wip_bytes_box);
    r.pubkey = unshiftDeclaredWidthBytesFromBytes(wip_bytes_box);
    r.pk_section_length = unshiftFixedWidthIntFromBytes(wip_bytes_box);
    r.checkints = unshiftBytesFromBytes(wip_bytes_box, 4);
    r.checkints2 = unshiftBytesFromBytes(wip_bytes_box, 4);
    r.pk_keytype = unshiftDeclaredWidthStringFromBytes(wip_bytes_box);
    r.pk_pubkey = unshiftDeclaredWidthBytesFromBytes(wip_bytes_box);
    r.private_key = unshiftDeclaredWidthBytesFromBytes(wip_bytes_box);
    r.comment = unshiftDeclaredWidthStringFromBytes(wip_bytes_box);
    r.final_bytes = wip_bytes_box.bytes.subarray(0);

    pem.cipher_name = r.cipher_name;
    pem.kdfname = r.kdfname;
    pem.kdfopts = r.kdfopts;
    pem.num_of_keys = r.num_of_keys;
    pem.key_type = r.key_type.replace("ssh-", "");
    pem.public_key = r.pubkey;
    pem.private_key = r.private_key;
    pem.comment = r.comment;
    pem.details = r;

    return pem;
  }

  toSSHDotPubString() {
    const pk_bytes = new Uint8Array([
      ...declaredWidthString(`ssh-${this.key_type}`),
      ...declaredWidthBytes(this.public_key),
    ]);
    const pk_hex = uint8ArrayToString(pk_bytes, "base64");
    return `ssh-${this.key_type} ${pk_hex} ${this.comment}`;
  }

  static fromSSHDotPubString(dot_pub_str) {
    const pem = new SSHPem();

    const str_split = dot_pub_str.split(" ");

    pem.key_type = str_split[0].replace("ssh-", "");

    const inner_pubkey_str = str_split[1];
    const inner_pubkey_uint8a = uint8ArrayFromString(
      inner_pubkey_str,
      "base64"
    );
    const pubkey_uint8a = new Uint8Array([...inner_pubkey_uint8a.subarray(19)]);
    pem.public_key = pubkey_uint8a;

    if (str_split.length > 1) {
      pem.comment = str_split[2];
    }

    return pem;
  }
}

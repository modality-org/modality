import { chacha12 } from "@noble/ciphers/chacha";
import { hexToBytes, utf8ToBytes } from "@noble/ciphers/utils";

// TODO - this is a placeholder key and data
const DEFAULT_KEY = utf8ToBytes("1234567890ABCDEF1234567890ABCDEF");
const DEFAULT_DATA = "LFGLFGLFGLFGLFG";

export default class ChaCha {
  constructor({ key = DEFAULT_KEY, data = DEFAULT_DATA } = {}) {
    this.key = key;
    this.data = data;
  }

  async pickOne({ input, options }) {
    input = input.substr(0, 12).padEnd(12, "\0");
    const nonce = utf8ToBytes(input);
    const data = utf8ToBytes(this.data);
    const result = chacha12(this.key, nonce, data);
    const result_int =
      result[0] +
      result[1] * 16 +
      result[2] * 16 * 16 +
      result[3] * 16 * 16 * 16;
    const i = result_int % options.length;
    return options[i];
  }
}

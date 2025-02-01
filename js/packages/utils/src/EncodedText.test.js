import { expect, describe, test } from "@jest/globals";

import { encode, decode, recode } from './EncodedText.js'

describe('EncodedText', () => {
  test('should encode, decode, and recode UTF8 strings', async () => {
    const text = "Hello 👋";

    const text_as_base64pad = encode(text, 'base64pad');
    expect(text_as_base64pad).toBe("SGVsbG8g8J+Riw==");
    const text_from_base64pad = decode(text_as_base64pad, 'base64pad');
    expect(text_from_base64pad).toBe(text);

    const text_as_base58btc = encode(text, 'base58btc');
    expect(text_as_base58btc).toBe("54uZdaj8RcuH6S");
    const text_from_base58btc = decode(text_as_base58btc, 'base58btc');
    expect(text_from_base58btc).toBe(text);

    const text_from_64_to_58 = recode(text_as_base64pad, 'base64pad', 'base58btc');
    expect(text_from_64_to_58).toBe("54uZdaj8RcuH6S");
  });
});
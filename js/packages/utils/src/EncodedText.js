import { toString as uint8ArrayToString } from "uint8arrays/to-string";
import { fromString as uint8ArrayFromString } from "uint8arrays/from-string";

export function encode(text, encoding) {
  return uint8ArrayToString(uint8ArrayFromString(text), encoding);
}

export function decode(text, encoding) {
  return uint8ArrayToString(uint8ArrayFromString(text, encoding));
}

export function recode(text, encoding1, encoding2) {
  return uint8ArrayToString(uint8ArrayFromString(text, encoding1), encoding2);
}
import * as HashFuncs from "hash-wasm";

const HASH_FUNC_HEXADECIMAL_LENGTH = {
  sha1: 40,
  sha256: 64,
  sha384: 96,
  sha512: 128,
};

const DEFAULT_MAX_TRIES = BigInt(100_000_000_000);
const DEFAULT_HASH_FUNC_NAME = "sha256";
const DEFAULT_DIFFICULTY_COEFFICIENT = 0xffff;
const DEFAULT_DIFFICULTY_EXPONENT = 0x1d;
const DEFAULT_DIFFICULTY_BASE = 8;

export async function mine({
  data,
  difficulty,
  maxTries = DEFAULT_MAX_TRIES,
  hashFuncName = DEFAULT_HASH_FUNC_NAME,
}) {
  let nonce = 0;

  let tryCount = 0;
  while (tryCount < maxTries) {
    tryCount++;
    let hash = await hashWithNonce({ data, nonce, hashFuncName });
    if (isHashAcceptable({ hash, difficulty, hashFuncName })) {
      return nonce;
    }
    nonce++;
  }
  throw new Error("maxTries reached, no nonce found");
}

export async function hashWithNonce({ data, nonce, hashFuncName = "sha256" }) {
  const hashFunc = HashFuncs[hashFuncName];
  const hash = await hashFunc(`${data}${nonce}`);
  return hash;
}

function difficultyToTargetHash({
  difficulty,
  hashFuncName = DEFAULT_HASH_FUNC_NAME,
  coefficient = DEFAULT_DIFFICULTY_COEFFICIENT,
  exponent = DEFAULT_DIFFICULTY_EXPONENT,
  base = DEFAULT_DIFFICULTY_BASE,
}) {
  const hexLength = HASH_FUNC_HEXADECIMAL_LENGTH[hashFuncName];
  coefficient = BigInt(coefficient);
  exponent = BigInt(exponent);
  base = BigInt(base);
  const max_target = coefficient << (exponent * base);
  const targetBigNum = BigInt(max_target) / BigInt(difficulty);
  const targetHash = targetBigNum.toString(16).padStart(hexLength, "0");
  return targetHash;
}

export function isHashAcceptable({
  hash,
  difficulty,
  hashFuncName = DEFAULT_HASH_FUNC_NAME,
}) {
  const targetHash = difficultyToTargetHash({ difficulty, hashFuncName });
  const targetBigInt = BigInt("0x" + targetHash);
  const hashBigInt = BigInt("0x" + hash);
  return hashBigInt < targetBigInt;
}

export async function validateNonce({
  data,
  nonce,
  difficulty,
  hashFuncName = "sha256",
}) {
  const hash = await hashWithNonce({ data, nonce, hashFuncName });
  return isHashAcceptable({ hash, difficulty, hashFuncName });
}

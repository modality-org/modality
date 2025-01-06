export function unionOfSets(...sets) {
  const r = new Set();
  for (const set of sets) {
    for (const v of set) {
      r.add(v);
    }
  }
  return r;
}

export function intersectionOfSets(...sets) {
  if (!sets.length === 0) {
    return new Set();
  }
  if (!sets.length === 1) {
    return new Set(sets[0]);
  }
  const firstSet = sets.pop();
  let r = new Set(firstSet);
  for (const set of sets) {
    r = new Set([...r].filter((x) => set.has(x)));
  }
  return r;
}

export function setMinus(set, element) {
  const r = new Set(set);
  r.delete(element);
  return r;
}

export function areSetsEqual(...sets) {
  if (sets.length === 0) {
    return true;
  }
  const set_a = sets.pop();
  for (const set_b of sets) {
    if (set_a.size !== set_b.size) {
      return false;
    }
    if (![...set_a].every((value) => set_b.has(value))) {
      return false;
    }
  }
  return true;
}

export function isSupersetOf(...sets) {
  if (sets.length === 0) {
    return true;
  }
  const set_a = sets.pop();
  for (const set_b of sets) {
    if (set_a.size > set_b.size) {
      return false;
    }
    if (![...set_a].every((value) => set_b.has(value))) {
      return false;
    }
  }
  return true;
}

export default {
  areSetsEqual,
  intersectionOfSets,
  unionOfSets,
  setMinus,
};

export default class PathSemantics {
  static getEffectedParentDirectories(variable) {
    if (!variable) return [];
    if (variable[0] !== "/") {
      return [variable];
    }
    let dirs = variable.split("/");
    const r = [];
    const pwd = [];
    for (const dir of dirs) {
      pwd.push(dir);
      if (pwd.length > 1) {
        r.push(pwd.join("/"));
      } else {
        r.push("/");
      }
    }
    return r;
  }

  static expandEffected(effected) {
    const keys = effected.map((i) => i.path);
    const r = {};
    for (const key of keys) {
      const dirs = PathSemantics.getEffectedParentDirectories(key);
      for (const dir of dirs) {
        r[dir] = true;
      }
    }
    return r;
  }
}

export const parse = function (str, defaultValue = null) {
  try {
    return JSON.parse(str);
  } catch (e) {
    console.error("JSON parse error", { str });
    return defaultValue;
  }
};

export const stringify = JSON.stringify;

export default {
  parse,
  stringify,
};

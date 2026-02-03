/**
 * PathValue - Typed values at contract paths
 */

/**
 * Path types
 */
export const PathType = {
  BOOL: '.bool',
  TEXT: '.text',
  JSON: '.json',
  ID: '.id',
  DATE: '.date',
  DATETIME: '.datetime',
  WASM: '.wasm',
  MODALITY: '.modality',
  MD: '.md',
};

/**
 * Infer path type from path string
 * @param {string} path
 * @returns {string|null}
 */
export function inferPathType(path) {
  for (const [, ext] of Object.entries(PathType)) {
    if (path.endsWith(ext)) {
      return ext;
    }
  }
  return null;
}

/**
 * Validate a value against a path type
 * @param {any} value
 * @param {string} pathType
 * @returns {boolean}
 */
export function validateValue(value, pathType) {
  switch (pathType) {
    case PathType.BOOL:
      return typeof value === 'boolean';
    case PathType.TEXT:
    case PathType.MD:
      return typeof value === 'string';
    case PathType.JSON:
      return value !== undefined;
    case PathType.ID:
      return typeof value === 'string' && /^[a-f0-9]{64}$/i.test(value);
    case PathType.DATE:
      return typeof value === 'string' && /^\d{4}-\d{2}-\d{2}$/.test(value);
    case PathType.DATETIME:
      return typeof value === 'string' && !isNaN(Date.parse(value));
    case PathType.WASM:
      return value instanceof Uint8Array || typeof value === 'string';
    case PathType.MODALITY:
      return typeof value === 'string';
    default:
      return true;
  }
}

/**
 * PathValue class for working with typed contract paths
 */
export class PathValue {
  /**
   * @param {string} path - The full path
   * @param {any} value - The value
   */
  constructor(path, value) {
    this.path = path;
    this.value = value;
    this.type = inferPathType(path);
  }

  /**
   * Get the directory part of the path
   * @returns {string}
   */
  get directory() {
    const lastSlash = this.path.lastIndexOf('/');
    return lastSlash > 0 ? this.path.slice(0, lastSlash) : '/';
  }

  /**
   * Get the filename part of the path
   * @returns {string}
   */
  get filename() {
    const lastSlash = this.path.lastIndexOf('/');
    return lastSlash >= 0 ? this.path.slice(lastSlash + 1) : this.path;
  }

  /**
   * Get the name without extension
   * @returns {string}
   */
  get name() {
    const filename = this.filename;
    const dotIndex = filename.lastIndexOf('.');
    return dotIndex > 0 ? filename.slice(0, dotIndex) : filename;
  }

  /**
   * Validate the value against the path type
   * @returns {boolean}
   */
  isValid() {
    if (!this.type) return true;
    return validateValue(this.value, this.type);
  }

  /**
   * Create a bool path value
   * @param {string} path - Path ending in .bool
   * @param {boolean} value
   * @returns {PathValue}
   */
  static bool(path, value) {
    if (!path.endsWith('.bool')) {
      path = path + '.bool';
    }
    return new PathValue(path, value);
  }

  /**
   * Create a text path value
   * @param {string} path - Path ending in .text
   * @param {string} value
   * @returns {PathValue}
   */
  static text(path, value) {
    if (!path.endsWith('.text')) {
      path = path + '.text';
    }
    return new PathValue(path, value);
  }

  /**
   * Create a JSON path value
   * @param {string} path - Path ending in .json
   * @param {object} value
   * @returns {PathValue}
   */
  static json(path, value) {
    if (!path.endsWith('.json')) {
      path = path + '.json';
    }
    return new PathValue(path, value);
  }

  /**
   * Create an ID path value
   * @param {string} path - Path ending in .id
   * @param {string} publicKeyHex - 64-char hex public key
   * @returns {PathValue}
   */
  static id(path, publicKeyHex) {
    if (!path.endsWith('.id')) {
      path = path + '.id';
    }
    return new PathValue(path, publicKeyHex);
  }

  /**
   * Create a modality path value
   * @param {string} path - Path ending in .modality
   * @param {string} modalityContent
   * @returns {PathValue}
   */
  static modality(path, modalityContent) {
    if (!path.endsWith('.modality')) {
      path = path + '.modality';
    }
    return new PathValue(path, modalityContent);
  }

  /**
   * Convert to JSON
   * @returns {object}
   */
  toJSON() {
    return {
      path: this.path,
      value: this.value,
      type: this.type,
    };
  }
}

export default PathValue;

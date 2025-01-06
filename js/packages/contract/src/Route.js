const ROUTE_PRIMITIVE_TYPES = [
  "text",
  "string",
  "boolean",
  "bool",
  "integer",
  "int",
  "number",
  "num",
  "balance",
  "bal",
  "set",
  "list",
  "table",
];
const ROUTE_ATTACHMENT_TYPES = [
  "txt",
  "json",
  "md",
  "pdf",
  "zip",
  "tar",
  "html",
  "png",
  "jpg",
  "jpeg",
  "gif",
  "svg",
  "doc",
  "docx",
  "xls",
  "xlsx",
  "csv",
];
const ROUTE_FILE_TYPES = [...ROUTE_PRIMITIVE_TYPES, ...ROUTE_ATTACHMENT_TYPES];

export default class Route {
  constructor(path) {
    this.path = path;
    return this;
  }

  static getPrimitiveTypes() {
    return ROUTE_PRIMITIVE_TYPES;
  }

  static getAttachmentTypes() {
    return ROUTE_ATTACHMENT_TYPES;
  }

  static getType(path) {
    if (!Route.isValidPath(path)) {
      return null;
    }
    if (Route.isFolder(path)) {
      return "folder";
    } else if (Route.isPrimitive(path)) {
      return "primitive";
    } else if (Route.isAttachment(path)) {
      return "attachment";
    } else {
      return null;
    }
  }

  static isFolder(path) {
    if (!Route.isValidPath(path)) {
      return null;
    }
    return !path.match(`\\.`);
  }

  static isValidPath(path) {
    const folderMatcher = `^\\/([\\/a-zA-Z0-9_\\-]+\\/)*([a-zA-Z0-9_\\-]+)$`;
    const fileMatcher = `^\\/([\\/a-zA-Z0-9_\\-]+\\/)*([a-zA-Z0-9_\\-]+)\\.([a-zA-Z0-9]+)$`;
    if (path.match(folderMatcher) || path.match(fileMatcher)) {
      return true;
    }
  }

  static getPathParts(path) {
    const folderMatcher = `^\\/([\\/a-zA-Z0-9_\\-]+\\/)*([a-zA-Z0-9_\\-]+)$`;
    const fileMatcher = `^\\/([\\/a-zA-Z0-9_\\-]+\\/)*([a-zA-Z0-9_\\-]+)\\.([a-zA-Z0-9]+)$`;
    let m;
    if ((m = path.match(fileMatcher))) {
      return {
        folder_path: `/${m[1]}`,
        file_name: m[2],
        file_type: m[3],
      };
    } else if ((m = path.match(folderMatcher))) {
      return {
        folder_path: `/${m[1]}`,
        file_name: null,
        file_type: null,
      };
    }
  }

  static isFile(path) {
    if (!Route.isValidPath(path)) {
      return false;
    }
    return path.match(`\\.(${ROUTE_FILE_TYPES.join("|")})$`);
  }

  static isPrimitive(path) {
    if (!Route.isValidPath(path)) {
      return false;
    }
    return path.match(`\\.(${ROUTE_PRIMITIVE_TYPES.join("|")})$`);
  }

  static isAttachment(path) {
    if (!Route.isValidPath(path)) {
      return false;
    }
    return path.match(`\\.(${ROUTE_ATTACHMENT_TYPES.join("|")})$`);
  }
}

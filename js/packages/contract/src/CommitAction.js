import Route from "./Route.js";
import { Expression as ModalityExpression } from "@modality-dev/formulas";

export const METHODS = [
  "post",
  "rule",
  // "define",
  // "repost",
  // "create",
  // "send",
  // "receive",
];

export default class CommitAction {
  constructor({ method, path, value }) {
    if (!METHODS.includes(method)) {
      throw new Error(`unknown method: ${method}`);
    }
    this.method = method;
    this.path = path;
    this.value = value;
    return this;
  }

  validateOrThrow() {
    if (this.method === "post") {
      if (!Route.isValidPath(this.path)) {
        throw new Error(`Invalid path ${this.path}`);
      }
      if (!Route.getType(this.path)) {
        throw new Error(`Cannot post to route ${this.path} \n
    You can only post to routes of known types. \n
    Primitive file types: ${Route.getPrimitiveTypes().join(", ")}
    Attachment file type: ${Route.getAttachmentTypes().join(", ")}

    For example: ${this.path}.text
    `);
      }
      return true;
    } else if (this.method === "rule") {
      try {
        const m = new ModalityExpression(this.value);
        const ef = m.expandFunctions();
        const props = Object.keys(ef.functions);
        for (const prop of props) {
          if (!prop.match("__")) {
            throw new Error(
              `Unrecognized prop used "${prop}". Please use one of the builtin test functions like include_sig or post_to.`
            );
          }
        }
      } catch (e) {
        throw new Error(`unable to parse rule: ${this.value}\n ${e}`);
      }
      return true;
    }
    throw new Error(`unknown method: ${this.method}`);
  }

  toJSON() {
    return {
      method: this.method,
      path: this.path,
      value: this.value,
    };
  }

  hasAttachment() {
    return this.value.match?.("^attachment://");
  }

  getFileHash() {
    if (this.hasAttachment()) {
      return this.value.substr("attachment://".length);
    }
    return null;
  }

  getPath() {
    return this.path;
  }
}

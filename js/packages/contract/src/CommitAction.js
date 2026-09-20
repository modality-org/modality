import Route from "./Route.js";
import { Expression as ModalityExpression } from "@modality-dev/formulas";

export const METHODS = [
  "post",
  "rule",
  "invoke",
  "repost",
  // "define",
  // "create",
  // "send",
  // "receive",
];

export default class CommitAction {
  constructor({ method, path, value, source_contract, source_path, source_commit }) {
    if (!METHODS.includes(method)) {
      throw new Error(`unknown method: ${method}`);
    }
    this.method = method;
    this.path = path;
    this.value = value;
    this.source_contract = source_contract;
    this.source_path = source_path;
    this.source_commit = source_commit;
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
    } else if (this.method === "invoke") {
      if (!this.path) {
        throw new Error("INVOKE action requires a path to the program");
      }
      if (!this.path.startsWith("/__programs__/") || !this.path.endsWith(".wasm")) {
        throw new Error("INVOKE action path must be /__programs__/{name}.wasm");
      }
      if (typeof this.value !== "object" || !this.value.args) {
        throw new Error("INVOKE action value must be an object with 'args' field");
      }
      return true;
    } else if (this.method === "repost") {
      if (!Route.isValidPath(this.path)) {
        throw new Error(`Invalid dest path ${this.path}`);
      }
      if (!this.source_contract) {
        throw new Error("REPOST requires source_contract");
      }
      if (!this.source_path || !Route.isValidPath(this.source_path)) {
        throw new Error(`REPOST requires a valid source_path, got ${this.source_path}`);
      }
      if (!this.source_commit) {
        throw new Error("REPOST requires source_commit");
      }
      return true;
    }
    throw new Error(`unknown method: ${this.method}`);
  }

  toJSON() {
    const json = {
      method: this.method,
      path: this.path,
      value: this.value,
    };
    if (this.source_contract) json.source_contract = this.source_contract;
    if (this.source_path) json.source_path = this.source_path;
    if (this.source_commit) json.source_commit = this.source_commit;
    return json;
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

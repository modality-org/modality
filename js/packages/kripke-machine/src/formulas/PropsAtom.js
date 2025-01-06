import BaseFormula from "./BaseFormula.js";
import { unionOfSets } from "@modality-dev/utils/sets";

export default class PropsAtom extends BaseFormula {
  constructor(signed_props) {
    super();
    this.props = signed_props;
  }

  async getValue(ctx) {
    return true; // TODO
  }

  getProps() {
    return new Set(this.props.map((i) => i.prop));
  }

  getSignedProps() {
    return new Set(this.props.map((i) => `${i.sign ? "+" : "-"}${i.prop}`));
  }

  getFreeVars() {
    return new Set(this.props.map((i) => i.props).filter((i) => i === "*"));
  }

  getBoundVars() {
    return new Set();
  }

  toText() {
    return `${this.props.map((sa) => sa.toText()).join(" ")}`;
  }
}

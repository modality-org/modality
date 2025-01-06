import BaseFormula from "./BaseFormula.js";

export default class PropsSet extends BaseFormula {
  constructor(signed_props) {
    super();
    this.props = signed_props;
  }

  async getValue(ctx) {
    return true; // TODO
  }

  expandFunctions() {
    const used_props = this.props.map((sa) => sa.toUnsignedText());
    return {
      constraint: "true",
      functions: used_props.reduce((acc, i) => {
        return {
          ...acc,
          [i]: true,
        };
      }, {}),
    };
  }

  getFreeVars() {
    return new Set(this.props.map((i) => i.props).filter((i) => i === "*"));
  }

  getBoundVars() {
    return new Set();
  }

  negated({ filterMaybe = false } = {}) {
    let props = [...this.props];
    if (filterMaybe) {
      props = [...this.props.filter((i) => !i.isMaybe())];
    }
    const pa = new PropsSet(props.map((sa) => sa.negated()));
    return pa;
  }

  toText() {
    return `${this.props.map((sa) => sa.toText()).join(" ")}`;
  }

  toModalFormula({ filterMaybe = false } = {}) {
    let props = [...this.props];
    if (filterMaybe) {
      props = [...this.props.filter((i) => !i.isMaybe())];
    }
    return `${props
      .map((sa) => sa.toText())
      .filter((i) => !!i.length)
      .join(" ")}`;
  }
}

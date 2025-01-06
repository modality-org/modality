import antlr4 from "antlr4";

import Parser from "../grammars/build/ModalFormulaParser.js";
import Lexer from "../grammars/build/ModalFormulaLexer.js";
import Visitor from "./Visitor.js";
import { unionOfSets, setMinus } from "@modality-dev/utils/sets";

import BaseFormula from "./BaseFormula.js";
import TrueAtom from "./TrueAtom.js";
import FalseAtom from "./FalseAtom.js";

export { default as PropsAtom } from "./PropsAtom.js";
export { default as TrueAtom } from "./TrueAtom.js";
export { default as FalseAtom } from "./FalseAtom.js";
export { default as AndFormula } from "./AndFormula.js";
export { default as OrFormula } from "./OrFormula.js";
export { default as NotFormula } from "./NotFormula.js";
export { default as BoxFormula } from "./BoxFormula.js";
export { default as DiamondFormula } from "./DiamondFormula.js";
export { default as GfpFormula } from "./GfpFormula.js";
export { default as LfpFormula } from "./LfpFormula.js";
export { default as BoundVar } from "./BoundVar.js";
class ThrowingErrorListener extends antlr4.error.ErrorListener {
  constructor() {
    super();
  }

  syntaxError(recognizer, offendingSymbol, line, column, msg, e) {
    throw new Error(
      `Parsing ERROR: ${offendingSymbol} line ${line}, col ${column}: ${msg}`
    );
  }
}

export class ModalFormula {
  constructor(input, defaultTo = null) {
    if (typeof input === "string") {
      if (input.match(/^\s*$/)) {
        return defaultTo ? new TrueAtom() : new FalseAtom();
      }
      const parser = this.constructor.parse(input);
      parser.removeErrorListeners();
      parser.addErrorListener(new ThrowingErrorListener());
      const formula = parser.formula();
      const visitor = new Visitor();
      return visitor.visit(formula)[0];
    } else if (input?.prototype instanceof BaseFormula) {
      return input;
    } else {
      throw new Error("not a formula");
    }
  }

  static parse(text) {
    const chars = new antlr4.InputStream(text);
    const lexer = new Lexer(chars);
    const tokens = new antlr4.CommonTokenStream(lexer);
    const parser = new Parser(tokens);
    parser.buildParseTrees = true;
    return parser;
  }
}

export default ModalFormula;

import antlr4 from "antlr4";

import Parser from "../grammars/build/ModalityParser.js";
import Lexer from "../grammars/build/ModalityLexer.js";
import Visitor from "./Visitor.js";

import BaseFormula from "./BaseFormula.js";
import TrueAtom from "./TrueAtom.js";
import FalseAtom from "./FalseAtom.js";
import PropsSet from "./PropsSet.js";

// propositions
export { default as PropAtom } from "./PropAtom.js";
export { default as PropsSet } from "./PropsSet.js";
export { default as SignedProp } from "./SignedProp.js";

// propositional logic
export { default as TrueAtom } from "./TrueAtom.js";
export { default as FalseAtom } from "./FalseAtom.js";
export { default as AndFormula } from "./AndFormula.js";
export { default as OrFormula } from "./OrFormula.js";
export { default as NotFormula } from "./NotFormula.js";
export { default as NegatedFormula } from "./NegatedFormula.js";

// temporal logic
export { default as BoxFormula } from "./BoxFormula.js";
export { default as DiamondFormula } from "./DiamondFormula.js";
export { default as GfpFormula } from "./GfpFormula.js";
export { default as LfpFormula } from "./LfpFormula.js";
export { default as StateSetVariable } from "./StateSetVariable.js";

// temporal logic macros
export { default as MustMacro } from "./MustMacro.js";
export { default as CanMacro } from "./CanMacro.js";
export { default as AlwaysMacro } from "./AlwaysMacro.js";
export { default as EventuallyMacro } from "./EventuallyMacro.js";
// ...
export { default as WhenAlsoFormula } from "./WhenAlsoFormula.js";
export { default as WhenNextFormula } from "./WhenNextFormula.js";

// other
export { default as FunctionAtom } from "./FunctionAtom.js";
export { default as Variable } from "./Variable.js";

class CustomErrorListener extends antlr4.error.ErrorListener {
  syntaxError(recognizer, offendingSymbol, line, column, msg, e) {
    throw new Error(
      `Parsing ERROR: ${offendingSymbol} line ${line}, col ${column}: ${msg}`
    );
  }

  reportAmbiguity(
    recognizer,
    dfa,
    startIndex,
    stopIndex,
    exact,
    ambigAlts,
    configs
  ) {}

  reportAttemptingFullContext(
    recognizer,
    dfa,
    startIndex,
    stopIndex,
    conflictingAlts,
    configs
  ) {}

  reportContextSensitivity(
    recognizer,
    dfa,
    startIndex,
    stopIndex,
    prediction,
    configs
  ) {}
}
export class Expression {
  constructor(input, defaultTo = null) {
    if (typeof input === "string") {
      if (input.match(/^\s*$/)) {
        return new PropsSet([]);
      }
      const parser = this.constructor.parse(input);
      parser.removeErrorListeners();
      parser.addErrorListener(new CustomErrorListener());
      const formula = parser.expression();
      const visitor = new Visitor();
      return visitor.visit(formula);
    } else if (input?.prototype instanceof BaseFormula) {
      return input;
    } else {
      throw new Error("not a formula");
    }
  }

  static parse(text) {
    const chars = new antlr4.InputStream(text);
    const lexer = new Lexer(chars);
    lexer.removeErrorListeners();
    lexer.addErrorListener(new CustomErrorListener());
    const tokens = new antlr4.CommonTokenStream(lexer);
    const parser = new Parser(tokens);
    parser.buildParseTrees = true;
    return parser;
  }
}

export default Expression;

import AbstractVisitor from "../grammars/build/ModalFormulaVisitor.js";

import {
  TrueAtom,
  FalseAtom,
  AndFormula,
  OrFormula,
  NotFormula,
  PropsAtom,
  BoxFormula,
  DiamondFormula,
  LfpFormula,
  GfpFormula,
  BoundVar,
} from "./ModalFormula.js";

import SignedProp from "./SignedProp.js";

export default class Visitor extends AbstractVisitor {
  visitTrueAtom(ctx) {
    return new TrueAtom();
  }

  visitFalseAtom(ctx) {
    return new FalseAtom();
  }

  visitNotFormula(ctx) {
    const formula = this.visit(ctx.inner);
    return new NotFormula(formula);
  }

  visitParenFormula(ctx) {
    return this.visit(ctx.inner);
  }

  visitOrFormula(ctx) {
    const left = this.visit(ctx.left);
    const right = this.visit(ctx.right);
    return new OrFormula(left, right);
  }

  visitAndFormula(ctx) {
    const left = this.visit(ctx.left);
    const right = this.visit(ctx.right);
    return new AndFormula(left, right);
  }

  visitPropsAtom(ctx) {
    const unsigned_actions = ctx.unsignedProp()
      ? [this.visit(ctx.unsignedProp())]
      : [];
    const signed_actions = Array.from(ctx.signedProp()).map((signed_action) => {
      const val = this.visit(signed_action);
      return val;
    });
    return new PropsAtom([...unsigned_actions, ...signed_actions]);
  }

  visitUnsignedProp(ctx) {
    const prop = this.visit(ctx.theProp);
    return new SignedProp(true, prop);
  }

  visitSignedProp(ctx) {
    const sign = ctx.theSign ? this.visit(ctx.theSign) : "+";
    const prop = this.visit(ctx.theProp);
    return new SignedProp(sign === "+", prop);
  }

  visitSign(ctx) {
    let str = ctx.getText();
    return str;
  }

  visitProp(ctx) {
    let str = ctx.getText();
    return str;
  }

  visitEmptyBoxFormula(ctx) {
    const inner = new PropsAtom([]);
    const outer = this.visit(ctx.outer);
    return new BoxFormula(inner, outer);
  }

  visitEmptyDiamondFormula(ctx) {
    const inner = new PropsAtom([]);
    const outer = this.visit(ctx.outer);
    return new DiamondFormula(inner, outer);
  }

  visitBoxFormula(ctx) {
    const inner = this.visit(ctx.inner);
    const outer = this.visit(ctx.outer);
    return new BoxFormula(inner, outer);
  }

  visitDiamondFormula(ctx) {
    const inner = this.visit(ctx.inner);
    const outer = this.visit(ctx.outer);
    return new DiamondFormula(inner, outer);
  }

  visitLfpFormula(ctx) {
    const bound_var = this.visit(ctx.boundVar);
    const inner = this.visit(ctx.inner);
    return new LfpFormula(bound_var, inner);
  }

  visitGfpFormula(ctx) {
    const bound_var = this.visit(ctx.boundVar);
    const inner = this.visit(ctx.inner);
    return new GfpFormula(bound_var, inner);
  }

  visitBoundVar(ctx) {
    let str = ctx.getText();
    return new BoundVar(str);
  }
}

import { expect, describe, it } from "@jest/globals";

import Expression from "./Expression.js";
import { getPropFromTest, escapeArgs } from "../TestFactory.js";
import Context from "../Context.js";

const VALID_FORMULAS = {
  [`false`]: `false`,
  [`true`]: `true`,
  [`[true] false`]: `[true] false`,
  [`<true> true`]: `<true> true`,
  [`+a`]: "+a",
  [`-a`]: "-a",
  [`?a`]: "",
  [`+a -b ?c`]: "+a -b",
  [`[+a] false`]: `[+a] false`,
  [`[-b] false`]: `[-b] false`,
  [`[?c] false`]: `[] false`,
  [`[+a -b ?c] false`]: `[+a -b] false`,
  [`[+a -b ?c] +a -b ?c`]: `[+a -b] +a -b`,
  [`must(+a -b ?c)`]: `[-a +b] false`,
  [`can(+a -b ?c)`]: `<+a -b> true`,
  [`always(must(+a))`]: `gfp(@x, [] @x and [-a] false)`,
  [`eventually(can(+a))`]: `lfp(@x, [] @x or <+a> true)`,
  [`post(/a.text)`]: `+post__${escapeArgs("/a.text")}`,
  [`must(post(/a.text))`]: `[-post__${escapeArgs("/a.text")}] false`,
  [`must(-post(/a.text))`]: `[+post__${escapeArgs("/a.text")}] false`,
  [`must(+post(/a.text) -post(/b.text) ?post(/c.text))`]: `[-post__${escapeArgs(
    "/a.text"
  )} +post__${escapeArgs("/b.text")}] false`,
};

describe("Expression", () => {
  it("should parse valid formulas", async () => {
    for (const [formula, validModalFormula] of Object.entries(VALID_FORMULAS)) {
      expect(() => {
        try {
          const expr = new Expression(formula);
          const mf = expr.toModalFormula();

          // run second time for consistent whitespace
          const expr2 = new Expression(mf);
          const mf2 = expr2.toModalFormula();

          // compare to valid modal formula
          const vmf_expr = new Expression(validModalFormula);
          const vmf_mf = vmf_expr.toModalFormula();
          expect(mf2).toBe(vmf_mf);
        } catch (e) {
          console.error(
            `Attempting to parse:   ${formula}\nAgainst modal formula: ${validModalFormula}`
          );
          throw e;
        }
      }).not.toThrow();
    }
  });

  it("should parse empty signed props, boxes, and diamonds", async () => {
    new Expression("");
    new Expression("[] true");
    new Expression("<> true");
  });

  it("should parse", async () => {
    let formula;

    expect(() => {
      new Expression(`<false`);
    }).toThrow();

    formula = new Expression(`<false> true`);
    expect(formula.constructor.name).toBe("DiamondFormula");
    expect(formula.inner.constructor.name).toBe("FalseAtom");
    expect(formula.outer.constructor.name).toBe("TrueAtom");

    formula = new Expression(`is(/here/this.md, "red")`);
    formula = formula.props[0].prop;
    expect(formula.constructor.name).toBe("FunctionAtom");
    expect(formula.args[0].constructor.name).toBe("Variable");
    expect(formula.args[1].constructor.name).toBe("String");

    formula = new Expression(`[+a -b ?c] false`);

    // formula = new Expression(`gfp(@x, [*]@x)`);
  });

  it.skip("Expression should parse", async () => {
    let formula, ctx, expandedFormula;
    formula = new Expression(`true and method_is("post")`);
    ctx = new Context({});
    expect(await formula.getValue(ctx)).toBe(false);
    ctx.setValue("method", "post");
    expect(await formula.getValue(ctx)).toBe(true);
    const methodIsPostPropName = getPropFromTest("method_is", ["post"]);
    expandedFormula = formula.expandFunctions();
    expect(expandedFormula.constraint).toBe(`true and ${methodIsPostPropName}`);
    expect(expandedFormula.functions[methodIsPostPropName]).not.toBeNull();
    // expect(await formula.getValue({ signedByMe: true })).toBe(true);

    // formula = new Expression("true and not ((not signedByMe))");
    // expect(await formula.getValue()).toBe(false);
    // expect(await formula.getValue({ signedByMe: true })).toBe(true);
  });

  it("should not parse formulas with trailing extra text", async () => {
    expect(() => {
      new Expression(`true and method_is("post") but also garbage`);
    }).toThrow();
  });
});

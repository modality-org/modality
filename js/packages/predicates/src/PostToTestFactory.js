import { TestFactory } from "@modality-dev/formulas";
import PathSemantics from "./PathSemantics.js";

export default class PostToTestFactory extends TestFactory {
  static name = "post_to";

  getEvaluateForArgs(args) {
    const [path] = args;
    return async (context) => {
      const { body } = context;
      const effected_paths = PathSemantics.expandEffected(body);
      return !!effected_paths[path];
    };
  }

  getCorrelateForArgs(args) {
    const [path] = args;
    return async (other_tests) => {
      const parent_dirs = PathSemantics.getEffectedParentDirectories(path);
      const r = [];
      for (const parent_dir of parent_dirs) {
        r.push({
          test: TestFactory.propFromNameAndArgs(PostToTestFactory.name, [
            parent_dir,
          ]),
          rules: [
            `always must (not(${TestFactory.propFromNameAndArgs(
              PostToTestFactory.name,
              [path]
            )}) or (${TestFactory.propFromNameAndArgs(PostToTestFactory.name, [
              path,
            ])} and ${TestFactory.propFromNameAndArgs(PostToTestFactory.name, [
              parent_dir,
            ])}))`,
          ],
        });
      }
      return r;
    };
  }
}

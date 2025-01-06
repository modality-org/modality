import Key from "@modality-dev/utils/Keypair";
import KripkeMachine, { Step } from "@modality-dev/kripke-machine";
import Modality from "@modality-dev/formulas";
import { unionOfSets } from "@modality-dev/utils/sets";
import {
  IncludeSigTestFactory,
  PostToTestFactory,
} from "@modality-dev/predicates";

import Commit from "./Commit.js";

KripkeMachine.setDefaultLanguage(Modality);
export default class Contract {
  constructor(genesis) {
    this.id = genesis?.genesis?.contract_id;
    this.genesis = genesis;
    this.commits = [];
    this.km = KripkeMachine.createLooper();
    this.km.language = Modality;
    return this;
  }

  clone() {
    const clone = new Contract(this.genesis);
    clone.commits = this.commits.map((c) => c.clone());
    clone.km = this.km.clone();
    return clone;
  }

  static async generate() {
    const signed_genesis = await this.generateGenesis();
    return new Contract(signed_genesis);
  }

  static async generateGenesis() {
    const key = await Key.generate();
    const contract_id = await key.asPublicAddress();
    const genesis = { contract_id };
    const signed_genesis = await key.signJSONElement({ genesis }, "genesis");
    return signed_genesis;
  }

  static async generateConfig() {
    return {};
  }

  async appendCommit(commit) {
    const observed_tests = Array.from(
      unionOfSets(...this.km.rules.map((rule) => rule.getProps()))
    ).map((p) => Modality.parseProp(p));
    const r = [];
    for (const observed_test of observed_tests) {
      // not actually a test
      if (!observed_test) {
        continue;
      }
      let tf;
      if (observed_test.name === "post_to") {
        tf = new PostToTestFactory("post_to");
      } else if (observed_test.name === "include_sig") {
        tf = new IncludeSigTestFactory("include_sig");
      }
      if (tf) {
        const t = tf.getTestForArgs(observed_test.args);
        const v = await t.evaluate({ body: commit.body, head: commit.head });
        r.push(
          `${v ? "+" : "-"}${Modality.getPropFromTest(
            observed_test.name,
            observed_test.args
          )}`
        );
      }
    }
    const props_text = r.join(" ");
    const rule = commit.getRulesConjoined();
    const rule_text = rule ? Modality.toModalFormula(rule) : null;
    const evolution_json = commit.getEvolutionJSON();
    const step = new Step(props_text, {
      rule_text,
      evolution_json,
    });
    this.km.takeStep(step);
    return this.commits.push(commit);
  }

  async appendCommitFromJson(commit_json) {
    const commit = Commit.fromJSON(commit_json);
    return await this.appendCommit(commit);
  }

  async canAppendCommit(commit) {
    const clone = this.clone();
    try {
      await clone.appendCommit(commit);
      return true;
    } catch (e) {
      console.error(e);
      return false;
    }
  }

  async canAppendCommitFromJson(commit_json) {
    const clone = this.clone();
    try {
      await clone.appendCommitFromJson(commit_json);
      return true;
    } catch (e) {
      console.error(e);
      return false;
    }
  }

  async appendCommitLog(commit_log) {
    for (const commit_json_string of commit_log) {
      const commit_json = JSON.parse(commit_json_string);
      await this.appendCommitFromJson(commit_json);
    }
    return this;
  }
}

import { TestFactory } from "@modality-dev/formulas";
import Key from "@modality-dev/utils/Keypair";

export default class IncludeSigTestFactory extends TestFactory {
  static name = "include_sig";

  getEvaluateForArgs(args) {
    const [prefixed_multiaddress] = args;
    const multiaddress = prefixed_multiaddress.replace(/^crypto:\//, "");
    return async (context) => {
      const { body, head } = context;
      if (!head?.signatures) {
        return false;
      }
      const signature = head.signatures[multiaddress];
      if (!signature) {
        return false;
      }

      const shared_key = Key.fromPublicMultiaddress(multiaddress);
      const result = await shared_key.verifySignatureForJson(signature, body);
      return result;
    };
  }

  getCorrelateForArgs(args) {
    return async (other_tests) => {
      return [];
    };
  }
}

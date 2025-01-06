import devnet_static1 from "./devnet-static1/index.js";
// import devnet_static2 from "./devnet-static2/index.js";
// import devnet_static3 from "./devnet-static3/index.js";
// import devnet_static4 from "./devnet-static4/index.js";
// import devnet_static5 from "./devnet-static5/index.js";

// import testnet from "./testnet/index.js";
import mainnet from "./mainnet/index.js";

export default class Networks {
  static getConfigFor(name) {
    switch (name) {
      case "devnet-static1":
        return devnet_static1;
      // case "devnet-static2":
      //   return devnet_static2;
      // case "devnet-static3":
      //   return devnet_static3;
      // case "devnet-static4":
      //   return devnet_static4;
      // case "devnet-static5":
      //   return devnet_static5;
      // case "testnet":
      //   return testnet;
      // case "mainnet":
      //   return mainnet;
      default:
        return null;
    }
  }
}

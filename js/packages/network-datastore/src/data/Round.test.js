import { jest, expect, describe, test, it } from "@jest/globals";

import Round from "./Round.js";
import NetworkDatastore from "../NetworkDatastore.js";

describe("Round", () => {
  it("should work", async () => {
    const datastore = await NetworkDatastore.createInMemory();
    let round;
    round = Round.from({ round: 1 });
    await round.save({ datastore });
    round = Round.from({ round: 2 });
    await round.save({ datastore });
    round = Round.from({ round: 3 });
    await round.save({ datastore });
    const max_round = await Round.findMaxId({ datastore });
    expect(max_round).toBe(3);
  });
});

import React from "react";
import styled from "styled-components";
import { Link } from "react-router-dom";

import Backend from "../lib/Backend.mjs";
import RoundRow from "../components/RoundRow.jsx";

export const layout = "HeaderFooter";

const INITIAL_MAX_ROUNDS = 20;
const SHOW_MORE_ROUNDS = 10;

export default function Page() {
  const [status, setStatus] = React.useState();
  const [minRound, setMinRound] = React.useState(0);
  const [maxRound, setMaxRound] = React.useState(0);

  React.useEffect(() => {
    (async () => {
      const r = await Backend.get("/");
      setStatus(r.data.status);
      setMaxRound(r.data.status.round);
      setMinRound(Math.max(r.data.status.round - INITIAL_MAX_ROUNDS, 1));
    })();
  }, []);

  const roundsToShow = Array.from(
    { length: maxRound === 0 ? 0 : maxRound - minRound + 1 },
    (_, i) => minRound + i
  ).reverse();

  return (
    <StyledDiv>
      <div to={`/rounds/${status?.round}`}>
        <div className="RoundRows">
          {roundsToShow.map((round) => (
            <RoundRow key={round} round={round} />
          ))}
        </div>
        {minRound > 1 && (
          <div
            className="show-more-rows"
            onClick={() => {
              setMinRound(Math.max(1, minRound - SHOW_MORE_ROUNDS));
            }}
          >
            show more
          </div>
        )}
      </div>
    </StyledDiv>
  );
}

const StyledDiv = styled.div/*css*/ `
  .RoundRows {
    display: flex;
    flex-direction: column;
    position: relative;
  }
  .RoundRow {
    display: flex;
    flex-direction: row;
    margin-bottom: 20px;
  }
  .show-more-rows {
    text-decoration: underline;
    margin-left: 40px;
    margin-right: 40px;
    width: calc(100% - 80px);
    cursor: pointer;
    color: blue;
  }
`;

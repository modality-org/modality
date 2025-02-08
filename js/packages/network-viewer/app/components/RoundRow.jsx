import React from "react";
import styled from "styled-components";
import { Link } from "react-router-dom";
import ScribeBlock from "./ScribeBlock";

import Backend from "../lib/Backend.mjs";

export const layout = "HeaderFooter";

export default function RoundRow({ round: round_number }) {
  const [round, setRound] = React.useState();
  const [scribes, setScribes] = React.useState([]);

  React.useEffect(() => {
    (async () => {
      const r = await Backend.get(`/rounds/${round_number}`);
      setRound(r.data.round);
      setScribes(r.data.round.scribes.sort());
    })();
  }, [round_number]);

  return (
    <StyledDiv className="RoundRow" id={`RoundRow-round-${round_number}`}>
      <div className="RoundInfo">
        <div>
          <a href={`/rounds/${round_number}`}>Round {round_number}</a>
        </div>
        <div>Scribes: {scribes.length}</div>
        <div>{round?.sequencing_method}</div>
      </div>
      {scribes.map((scribe) => (
        <ScribeBlock key={scribe} round={round_number} scribe={scribe} />
      ))}
      {scribes.length === 0 && <div className="not-available"></div>}
    </StyledDiv>
  );
}

const StyledDiv = styled.div/*css*/ `
  .RoundInfo {
    width: 150px;
    display: flex;
    flex-shrink: 0;
    flex-grow: 0;
    justify-content: center;
    flex-direction: column;
    align-items: center;
  }
  .not-available {
    width: 85px;
    height: 110px;
  }
`;

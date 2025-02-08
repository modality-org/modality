import React from "react";
import styled from "styled-components";
import { Link, useParams } from "react-router-dom";

import Backend from "../../../lib/Backend.mjs";
import RoundRow from "../../../components/RoundRow.jsx";

export const layout = "HeaderFooter";

export default function Block() {
  const [round, setRound] = React.useState();
  const { round_number: round_number_str } = useParams();
  const round_number = parseInt(round_number_str);

  React.useEffect(() => {
    (async () => {
      const r = await Backend.get(`/rounds/${round_number}`);
      setRound(r.data.page);
    })();
  }, [round_number]);

  return (
    <StyledDiv>
      <div>
        <a href={`/rounds/${round_number}`}>Round {round_number}</a>
      </div>
      <div className="RoundRows">
        {<RoundRow key={round_number + 1} round={round_number + 1} />}
        <RoundRow key={round_number} round={round_number} />
        {round_number > 1 && (
          <RoundRow key={round_number - 1} round={round_number - 1} />
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
`;

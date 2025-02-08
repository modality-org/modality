import React from "react";
import styled from "styled-components";
import { Link } from "react-router-dom";
import LinkerLine from "linkerline";

import Backend from "../lib/Backend.mjs";

export const layout = "HeaderFooter";

export default function ScribeBlock({ round, scribe, showLines = true }) {
  const [page, setBlock] = React.useState([]);
  const [lines, setLines] = React.useState([]);
  const [isHovering, setIsHovering] = React.useState(false);

  React.useEffect(() => {
    (async () => {
      const r = await Backend.get(`/rounds/${round}/scribes/${scribe}`);
      setBlock(r.data.page);
    })();
  }, [round]);

  React.useEffect(() => {
    const lr_certs = Object.values(page.last_round_certs || {});
    const connections = lr_certs.map((cert) => [
      { round: round - 1, scribe: cert.scribe },
      { round, scribe },
    ]);
    const _lines = [];
    for (const conn of connections) {
      const node1 = this.document.getElementById(
        `scribe-page-round-${conn[0].round}-scribe-${conn[0].scribe}`
      );
      const node2 = this.document.getElementById(
        `scribe-page-round-${conn[1].round}-scribe-${conn[1].scribe}`
      );
      if (node1 && node2) {
        let line = new LinkerLine({
          end: node1,
          start: node2,
          color: "blue",
          size: 1,
          startSocket: "bottom",
          endSocket: "top",
          startPlug: "behind",
          endPlug: "arrow3",
          path: "straight",
        });
        if (!showLines && !isHovering) {
          line.hide();
        }
        _lines.push(line);
      }
    }
    setLines(_lines);
  }, [page, showLines, isHovering]);

  return (
    <StyledDiv
      id={`scribe-page-round-${round}-scribe-${scribe}`}
      onMouseEnter={() => setIsHovering(true)}
      on
      onMouseLeave={() => setIsHovering(false)}
    >
      <Link to={`/rounds/${round}/scribes/${scribe}`}>
        <div className="Block">
          {page?.is_section_leader && <div className="section-leader">§</div>}
          {page?.is_certified && <div className="certified">✓</div>}
          <div className="scribe-id">...{page.scribe?.substr(-6)}</div>
          {page?.page_number && <div className="sequenced">#</div>}
        </div>
      </Link>
    </StyledDiv>
  );
}

const StyledDiv = styled.div/*css*/ `
  margin: 20px;
  svg.linker-line {
    stroke: green !important;
  }
  .scribe-id {
    borer: 1px solid #ccc;
    width: 100%;
    margin: 0px auto;
    margin-top: 45px;
    text-align: center;
    font-family: courier;
    font-size: 13px;
  }
  .Block {
    border: 1px solid #ccc;
    width: 85px;
    height: 110px;
    min-width: 85px;
    position: relative;
    .section-leader,
    .certified,
    .sequenced {
      position: absolute;
    }
    .section-leader {
      top: 5px;
      left: 5px;
    }
    .certified {
      bottom: 5px;
      left: 5px;
    }
    .sequenced {
      bottom: 5px;
      right: 5px;
    }
  }
`;

import React from "react";
import styled from "styled-components";
import { Link, useParams } from "react-router-dom";

import Backend from "../../../../../lib/Backend.mjs";

export const layout = "HeaderFooter";

export default function Block() {
  const [page, setBlock] = React.useState();
  const { round_number, scribe_id } = useParams();

  React.useEffect(() => {
    (async () => {
      const r = await Backend.get(
        `/rounds/${round_number}/scribes/${scribe_id}`
      );
      setBlock(r.data.page);
    })();
  }, [round_number, scribe_id]);

  return (
    <StyledDiv>
      <div>
        <a href={`/rounds/${round_number}`}>Round {round_number}</a>
      </div>
      <div>Scribe: {scribe_id}</div>
      <br />
      <div>
        Is Ordered: {page?.page_number ? "true" : "false"}
        <br />
        Block Number: {page?.page_number}
      </div>
      <br />
      <div>
        <div>Certificate: {page?.cert}</div>
        <br />
        <div>
          Last Round Certs:
          {Object.values(page?.last_round_certs || {})
            .sort((a, b) => a.scribe.localeCompare(b.scribe))
            .map((lrc) => (
              <div key={`${lrc.scribe}`}>
                * Scribe:{" "}
                <a href={`/rounds/${page.round - 1}/scribes/${lrc.scribe}`}>
                  {lrc.scribe}
                </a>{" "}
                | Cert: {lrc.cert}
              </div>
            ))}
        </div>
        <br />
        <div>
          Acks:
          {Object.values(page?.acks || {})
            .sort((a, b) => a.scribe.localeCompare(b.acker))
            .map((ack) => (
              <div key={`${ack.acker}`}>
                * Scribe:{" "}
                <a href={`/rounds/${ack.round}/scribes/${ack.acker}`}>
                  {ack.acker}
                </a>{" "}
                | Sig: {ack.acker_sig}
              </div>
            ))}
        </div>
        <br />
        <div>
          Late Acks:
          {page?.late_acks
            ?.sort((a, b) => a.scribe.localeCompare(b.scribe))
            .map((ack) => (
              <div key={`${ack.round}-${ack.scribe}`}>
                * Round {ack.round} from {ack.scribe}
              </div>
            ))}
        </div>
      </div>
      <br />
      <div>Is Section Leader: {page?.is_section_leader ? "true" : "false"}</div>
      <br />
    </StyledDiv>
  );
}

const StyledDiv = styled.div/*css*/ ``;

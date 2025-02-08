import React from "react";
import { Outlet, Link, useParams } from "react-router-dom";

export default function HeaderFooter() {
  const params = useParams();

  return (
    <div className="HeaderFooter Layout">
      <header>
        <h1>
          <Link to={"/"}>Modality Network Viewer</Link>
        </h1>
      </header>
      <main>
        <Outlet />
      </main>
    </div>
  );
}

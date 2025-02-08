import * as React from "react";
import * as ReactDOM from "react-dom/client";

import AppRouter from "@thylacine-js/webapp/AppRouter.mjs";
import createRoutes from "@thylacine-js/webapp/createRoutes.mjs";

async function main() {
  // eslint-disable-next-line no-undef
  const routes = await createRoutes(ROUTES_LIST);
  // eslint-disable-next-line no-undef
  const root = ReactDOM.createRoot(document.getElementById("root"));
  root.render(
    <React.StrictMode>
      <AppRouter routes={routes} />
    </React.StrictMode>
  );
}

main();

import "@fontsource-variable/bricolage-grotesque/wght.css";
import "@fontsource-variable/hanken-grotesk/wght.css";
import "./styles/emoji.css";
import "./styles/index.css";

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./app/App";
import { BootError } from "./app/BootError";
import { api, IpcFailure } from "./lib/ipc";

const container = document.getElementById("root");
if (!container) throw new Error("Missing #root element");
const root = createRoot(container);

api
  .bootstrap()
  .then((bootstrap) => {
    root.render(
      <StrictMode>
        <App bootstrap={bootstrap} />
      </StrictMode>,
    );
  })
  .catch((error: unknown) => {
    const message = error instanceof IpcFailure ? error.message : "The app's backend didn't respond.";
    root.render(<BootError message={message} />);
    void api.appReady();
  });

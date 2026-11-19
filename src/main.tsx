import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import Workspace from "./workspace/Workspace";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <Workspace />
  </StrictMode>,
);

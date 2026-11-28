import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { ArayuzSaglayici } from "../ortak";
import "../stil.css";
import WebApp from "./WebApp";

createRoot(document.getElementById("kok")!).render(
  <StrictMode>
    <ArayuzSaglayici>
      <WebApp />
    </ArayuzSaglayici>
  </StrictMode>,
);

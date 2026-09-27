import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import "./styles.css";

if (import.meta.env.DEV && new URLSearchParams(window.location.search).get("workbenchFixture") === "1") {
  void import("./workbench/visualFixture").then(module => module.mountVisualFixture());
} else {
  createRoot(document.getElementById("root")!).render(<StrictMode><App /></StrictMode>);
}

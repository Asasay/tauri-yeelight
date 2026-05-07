import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { App } from "./App";
import "./index.css";

const appWindow = getCurrentWindow();

appWindow.onResized(async () => {
  if (await appWindow.isMinimized()) {
    await appWindow.hide();
  }
});

createRoot(document.getElementById("root") as HTMLElement).render(
  <StrictMode>
    <App />
  </StrictMode>
);

import { lazy, StrictMode, Suspense } from "react";
import { createRoot } from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import App from "./App";
import "./index.css";

// Cửa sổ avatar nạp mã Live2D riêng (lazy) → cửa sổ chat không phụ thuộc Cubism Core.
const AvatarView = lazy(() => import("./AvatarView"));

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    {getCurrentWindow().label === "avatar" ? (
      <Suspense>
        <AvatarView />
      </Suspense>
    ) : (
      <App />
    )}
  </StrictMode>,
);

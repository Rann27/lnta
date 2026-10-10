import { AppRouter } from "./Router";
import { DevGallery } from "./DevGallery";

export function App() {
  const isGallery =
    import.meta.env.DEV &&
    typeof window !== "undefined" &&
    new URLSearchParams(window.location.search).get("view") === "gallery";

  if (isGallery) {
    return <DevGallery />;
  }

  return <AppRouter />;
}

export default App;

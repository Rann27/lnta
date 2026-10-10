import { DevGallery } from "./DevGallery";
import { PreviewBanner } from "./PreviewBanner";

export function App() {
  if (import.meta.env.DEV) {
    return (
      <>
        <PreviewBanner />
        <DevGallery />
      </>
    );
  }
  return (
    <div id="app">
      <h1>LNTA</h1>
    </div>
  );
}

export default App;

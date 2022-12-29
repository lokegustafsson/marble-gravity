import init, { initThreadPool, computeAccelsInner } from "./nbody.js";
(async () => {
  await init();
  await initThreadPool();
  self.onmessage = async (event) => {
    const bodies_bytes = event.data;
    const accels_bytes = await computeAccelsInner(bodies_bytes);
    self.postMessage(accels_bytes);
  };
  console.log("Rayon worker loaded");
})();

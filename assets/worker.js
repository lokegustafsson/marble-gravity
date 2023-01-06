import init, { initThreadPool, computeAccelsInner } from "./nbody.js";
(async () => {
  await init();
  console.info(`RAYON THREAD POOL size=${navigator.hardwareConcurrency}`);
  await initThreadPool(navigator.hardwareConcurrency);
  self.onmessage = async (event) => {
    const [id, payload] = event.data;
    if (payload == "ready?") {
      self.postMessage([id, "ready!"]);
      return;
    }
    const bodies_bytes = payload;
    const accels_bytes = await computeAccelsInner(bodies_bytes);
    self.postMessage([id, accels_bytes]);
  };
  console.info("RAYON WORKER LOADED");
})();

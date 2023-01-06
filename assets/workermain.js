import init, { initThreadPool, workerInner } from "./worker.js";
(async () => {
  await init();
  console.info(`RAYON THREAD POOL size=${navigator.hardwareConcurrency}`);
  await initThreadPool(navigator.hardwareConcurrency);
  self.onmessage = async (event) => {
    const [id, input] = event.data;
    if (input === "ready?") {
      self.postMessage([id, "ready!"]);
      return;
    }
    const output = await workerInner(input);
    self.postMessage([id, output], [output.buffer]);
  };
  console.info("ALL RAYON WORKERS LOADED");
})();

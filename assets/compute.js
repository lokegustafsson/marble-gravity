const worker = new Worker("./worker.js", { type:"module" });
export function computeAccelsOuter(bodies_bytes) {
  return new Promise((resolve, _reject) => {
    worker.onmessage = (event) => {
      resolve(event.data);
    };
    worker.postMessage(bodies_bytes);
  });
}

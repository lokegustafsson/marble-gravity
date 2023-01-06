const worker = new Worker("./worker.js", { type: "module" });
let ready = false;
let sequence_id = 0;
let id_to_resolver = {};
worker.onmessage = (event) => {
  const [id, payload] = event.data;
  if (!(id in id_to_resolver)) {
    console.error("main thread received unexpected message " + event);
    return;
  }
  const resolver = id_to_resolver[id];
  delete id_to_resolver[id];
  resolver(payload);
};
export function pollReady() {
  if (ready) {
    return true;
  } else {
    const id = ++sequence_id;
    id_to_resolver[id] = (reply) => {
      if (reply !== "ready!") {
        throw `unexpected reply ${reply}`
      }
      ready = true;
    };
    worker.postMessage([id, "ready?"]);
    console.log(`main->worker ready? poll=${id}`)
    return false;
  }
}
export function computeAccelsOuter(bodies_bytes) {
  return new Promise((resolve, _reject) => {
    const id = ++sequence_id;
    id_to_resolver[id] = resolve;
    worker.postMessage([id, bodies_bytes]);
  });
}

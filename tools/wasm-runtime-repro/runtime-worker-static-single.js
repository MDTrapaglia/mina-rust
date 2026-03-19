import * as singleModule from "./pkg-single/wasm_runtime_repro.js";

const workerUrl = new URL(self.location.href);

function trace(stage, details = undefined) {
  const url = new URL("/wasm-smoke/trace", self.location.origin);
  url.searchParams.set("stage", `runtime-worker:${stage}`);
  url.searchParams.set(
    "details",
    JSON.stringify({
      href: self.location.href,
      workerEntry: workerUrl.searchParams.get("worker_entry") ?? "static_single",
      ...(details ?? {}),
    }),
  );
  fetch(url, { cache: "no-store" }).catch(() => undefined);
}

let started = false;

self.addEventListener("message", event => {
  if (started || event.data?.type !== "start") {
    return;
  }
  started = true;
  trace("static.import.complete", {
    target: "single",
    exportKeys: Object.keys(singleModule).sort(),
  });
  self.postMessage({
    type: "complete",
    workerEntry: "static_single",
    target: "single",
    exportKeys: Object.keys(singleModule).sort(),
  });
});

const workerUrl = new URL(self.location.href);

const positiveInt = (name, fallback) => {
  const raw = Number(workerUrl.searchParams.get(name) ?? String(fallback));
  return Number.isFinite(raw) && raw > 0 ? Math.round(raw) : fallback;
};

const optionalPositiveInt = name => {
  const raw = Number(workerUrl.searchParams.get(name) ?? "");
  return Number.isFinite(raw) && raw > 0 ? Math.round(raw) : null;
};

const backend = workerUrl.searchParams.get("backend") ?? "js_webcrypto";
const wasmProfile = workerUrl.searchParams.get("wasm_profile") ?? "threaded";
const inputMode = workerUrl.searchParams.get("input_mode") ?? "js_copy";
const esmTarget = workerUrl.searchParams.get("esm_target") ?? "trivial";
const sizeBytes = positiveInt("size_bytes", 3317098);
const chunkSize = positiveInt("chunk_size", 16384);
const yieldEveryChunks = positiveInt("yield_every_chunks", 4);
const progressStep = positiveInt("progress_step", 1048576);
const forcedHardwareConcurrency = optionalPositiveInt(
  "forced_hardware_concurrency",
);

function trace(stage, details = undefined) {
  const url = new URL("/wasm-smoke/trace", self.location.origin);
  url.searchParams.set("stage", `runtime-worker:${stage}`);
  const mergedDetails = {
    href: self.location.href,
    backend,
    wasmProfile,
    inputMode,
    ...(details ?? {}),
  };
  url.searchParams.set("details", JSON.stringify(mergedDetails));
  fetch(url, { cache: "no-store" }).catch(() => undefined);
}

function installHardwareConcurrencyOverride(value) {
  if (!Number.isFinite(value) || value <= 0) {
    return false;
  }
  const targets = [];
  if (globalThis.Navigator?.prototype) {
    targets.push(globalThis.Navigator.prototype);
  }
  if (globalThis.navigator) {
    targets.push(globalThis.navigator);
  }
  for (const target of targets) {
    try {
      Object.defineProperty(target, "hardwareConcurrency", {
        configurable: true,
        get: () => value,
      });
      return true;
    } catch (_) {
      continue;
    }
  }
  return false;
}

function makePatternBytes(length) {
  const bytes = new Uint8Array(length);
  for (let index = 0; index < bytes.length; index += 1) {
    bytes[index] = index & 0xff;
  }
  return bytes;
}

function digestByteLength(digest) {
  if (digest instanceof ArrayBuffer) {
    return digest.byteLength;
  }
  if (ArrayBuffer.isView(digest)) {
    return digest.byteLength;
  }
  if (digest && typeof digest.length === "number") {
    return digest.length;
  }
  return null;
}

function importTargetRoot(target) {
  if (target === "single") {
    return "/tools/wasm-runtime-repro/pkg-single/wasm_runtime_repro.js";
  }
  if (target === "threaded") {
    return "/tools/wasm-runtime-repro/pkg/wasm_runtime_repro.js";
  }
  if (target === "trivial") {
    return "/tools/wasm-runtime-repro/probe-module.js";
  }
  throw new Error(`unsupported import target: ${target}`);
}

async function loadWasmBindings() {
  const pkgRoot = importTargetRoot(wasmProfile);
  trace("wasm.bindings.import.begin", {
    pkgRoot,
  });
  const module = await import(pkgRoot);
  trace("wasm.bindings.import.complete", {
    pkgRoot,
  });
  return {
    init: module.default,
    sha256_one_shot_js_input: module.sha256_one_shot_js_input,
    sha256_chunked_js_input: module.sha256_chunked_js_input,
    sha256_chunked_yielding_js_input: module.sha256_chunked_yielding_js_input,
    sha256_one_shot_generated: module.sha256_one_shot_generated,
    sha256_chunked_generated: module.sha256_chunked_generated,
    sha256_chunked_yielding_generated: module.sha256_chunked_yielding_generated,
  };
}

async function runProbe() {
  const startedAt = self.performance?.now?.() ?? Date.now();
  trace("start", {
    sizeBytes,
    chunkSize,
    yieldEveryChunks,
    progressStep,
    wasmProfile,
    forcedHardwareConcurrency,
  });

  if (forcedHardwareConcurrency !== null) {
    const installed = installHardwareConcurrencyOverride(
      forcedHardwareConcurrency,
    );
    trace(
      installed
        ? `hardware-concurrency.override.${forcedHardwareConcurrency}`
        : `hardware-concurrency.override_failed.${forcedHardwareConcurrency}`,
    );
  }

  let digest;
  if (backend === "js_webcrypto") {
    const bytes = makePatternBytes(sizeBytes);
    trace("js-webcrypto.begin", {
      byteLength: bytes.byteLength,
    });
    digest = await self.crypto.subtle.digest("SHA-256", bytes);
    trace("js-webcrypto.complete", {
      digestByteLength: digest.byteLength,
    });
  } else {
    if (backend === "esm_import_only") {
      const moduleRoot = importTargetRoot(esmTarget);
      trace("esm.import.begin", {
        esmTarget,
        moduleRoot,
      });
      const module = await import(moduleRoot);
      trace("esm.import.complete", {
        esmTarget,
        moduleRoot,
        exportKeys: Object.keys(module).sort(),
      });
      digest = new Uint8Array(0);
    } else {
    const bindings = await loadWasmBindings();
    trace("wasm.bindings.loaded", {
      wasmProfile,
    });

    if (backend === "wasm_import_only") {
      digest = new Uint8Array(0);
    } else if (wasmProfile === "threaded") {
      const memory = new WebAssembly.Memory({
        initial: 256,
        maximum: 65536,
        shared: true,
      });
      trace("wasm.memory.created", {
        initial: 256,
        maximum: 65536,
        shared: true,
      });
      trace("wasm.init.begin", {
        wasmProfile,
      });
      await bindings.init(undefined, memory);
      trace("wasm.init.complete", {
        shared: memory.buffer instanceof SharedArrayBuffer,
      });
    } else {
      trace("wasm.init.begin", {
        wasmProfile,
      });
      await bindings.init();
      trace("wasm.init.complete", {
        shared: false,
      });
    }

    if (backend === "wasm_import_only") {
      // Import-only probe: stop after module evaluation/loading.
    } else if (backend === "wasm_one_shot") {
      digest =
        inputMode === "wasm_generated"
          ? bindings.sha256_one_shot_generated(sizeBytes)
          : bindings.sha256_one_shot_js_input(makePatternBytes(sizeBytes));
    } else if (backend === "wasm_chunked") {
      digest =
        inputMode === "wasm_generated"
          ? bindings.sha256_chunked_generated(sizeBytes, chunkSize, progressStep)
          : bindings.sha256_chunked_js_input(
              makePatternBytes(sizeBytes),
              chunkSize,
              progressStep,
            );
    } else if (backend === "wasm_chunked_yielding") {
      digest =
        inputMode === "wasm_generated"
          ? await bindings.sha256_chunked_yielding_generated(
              sizeBytes,
              chunkSize,
              yieldEveryChunks,
              progressStep,
            )
          : await bindings.sha256_chunked_yielding_js_input(
              makePatternBytes(sizeBytes),
              chunkSize,
              yieldEveryChunks,
              progressStep,
            );
    } else {
      throw new Error(`unsupported backend: ${backend}`);
    }
    }
  }

  const elapsedMs = Math.round((self.performance?.now?.() ?? Date.now()) - startedAt);
  const message = {
    type: "complete",
    backend,
    wasmProfile,
    inputMode,
    elapsedMs,
    digestByteLength: digestByteLength(digest),
  };
  trace("complete", message);
  self.postMessage(message);
}

self.addEventListener("error", event => {
  trace("global.error", {
    message: event.message ?? null,
    filename: event.filename ?? null,
    lineno: event.lineno ?? null,
    colno: event.colno ?? null,
  });
});

self.addEventListener("unhandledrejection", event => {
  trace("global.unhandledrejection", {
    reason:
      event.reason?.message ??
      (event.reason === undefined ? null : String(event.reason)),
  });
});

let started = false;

self.addEventListener("message", event => {
  if (started || event.data?.type !== "start") {
    return;
  }
  started = true;
  runProbe().catch(error => {
    const message = {
      type: "error",
      backend,
      inputMode,
      message: error instanceof Error ? error.message : String(error),
      stack: error instanceof Error ? error.stack ?? null : null,
    };
    trace("error", message);
    self.postMessage(message);
  });
});

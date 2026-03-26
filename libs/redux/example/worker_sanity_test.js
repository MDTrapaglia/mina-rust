// Example sanity test for WASM worker clock sync
// Usage: ensure your WASM exposes the needed JS bindings, build with wasm-pack, and serve your pkg folder.
// Run in browser console or as part of a simple static site...

async function spawnWorkersWithDelay(numWorkers, delayMs) {
  const startSystem = Date.now();
  const startMonotonic = performance.now();

  for (let i = 0; i < numWorkers; i++) {
    const worker = new Worker('worker_entry.js');
    worker.postMessage({ startMonotonic, startSystem });
    worker.onmessage = (e) => console.log(`WORKER #${i} LOG:`, e.data);
    // Espera antes de crear el próximo worker
    await new Promise(r => setTimeout(r, delayMs));
  }
}

// Llamar así para test: (p.ej. 3 workers, 500ms de delay entre cada uno)
// spawnWorkersWithDelay(3, 500);
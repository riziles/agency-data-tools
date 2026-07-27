<script lang="ts">
  import { onMount } from 'svelte';
  import { browser } from '$app/environment';
  import { goto } from '$app/navigation';
  import { getQueryResult } from '$lib/stores.svelte';
  import { tableToIPC } from 'apache-arrow';

  // v3.8 Perspective — import WASM URLs via Vite's ?url
  import perspective, { init_server } from '@finos/perspective';
  import perspective_viewer, { init_client } from '@finos/perspective-viewer';
  import '@finos/perspective-viewer-datagrid';
  import '@finos/perspective-viewer-d3fc';
  import '@finos/perspective-viewer/dist/css/pro.css';

  import SERVER_WASM from '@finos/perspective/dist/wasm/perspective-server.wasm?url';
  import CLIENT_WASM from '@finos/perspective-viewer/dist/wasm/perspective-viewer.wasm?url';

  let loading = $state(true);
  let error = $state('');
  let info = $state({ sql: '', elapsedMs: 0, rowCount: 0 });
  let initialized = false;

  onMount(async () => {
    if (!browser) return;

    const result = getQueryResult();
    if (!result) { goto('/'); return; }

    info = { sql: result.sql, elapsedMs: result.elapsedMs, rowCount: result.table.numRows };

    try {
      // Init Perspective WASM (once)
      if (!initialized) {
        await Promise.all([
          init_server(fetch(SERVER_WASM)),
          init_client(fetch(CLIENT_WASM)),
        ]);
        initialized = true;
      }

      // Convert Arrow to IPC buffer
      const ipcData = tableToIPC(result.table);
      const buffer = ipcData.buffer.slice(ipcData.byteOffset, ipcData.byteOffset + ipcData.byteLength);

      // Create Perspective worker + table
      const worker = await perspective.worker();
      const table = await worker.table(buffer);

      // Load into viewer
      const viewer = document.querySelector('#perspective-viewer') as any;
      if (!viewer) throw new Error('Viewer element not found');

      const columns = result.table.schema.fields.map((f: any) => f.name);
      await viewer.load(table);
      await viewer.restore({
        plugin: 'Datagrid',
        theme: 'Pro Dark',
        settings: false,
        columns,
      });
    } catch (e: any) {
      error = e.message || String(e);
    } finally {
      loading = false;
    }
  });
</script>

<svelte:head>
  <title>Query Results — Fannie Mae</title>
</svelte:head>

<div class="viewer-page">
  <header>
    <a href="/" class="back-link">← Back to editor</a>
    <div class="meta">
      <span class="row-count">{info.rowCount.toLocaleString()} rows</span>
      <span class="elapsed">{info.elapsedMs}ms</span>
    </div>
  </header>

  {#if loading}
    <div class="loading">Loading Perspective...</div>
  {/if}
  {#if error}
    <div class="error">{error}</div>
  {/if}

  <div class="viewer-wrapper" class:hidden={loading}>
    <perspective-viewer
      id="perspective-viewer"
      style="height: calc(100vh - 60px); width: 100%;"
    ></perspective-viewer>
  </div>
</div>

<style>
  .viewer-page { display: flex; flex-direction: column; height: 100vh; background: var(--bg); }
  header { display: flex; justify-content: space-between; align-items: center; padding: 0.6rem 1.2rem; background: var(--surface); border-bottom: 1px solid var(--border); flex-shrink: 0; }
  .back-link { color: var(--primary); text-decoration: none; font-size: 0.9rem; }
  .back-link:hover { text-decoration: underline; }
  .meta { display: flex; gap: 1rem; font-size: 0.8rem; color: var(--muted); }
  .row-count { color: var(--text); font-weight: 600; }
  .loading, .error { position: absolute; top: 50%; left: 50%; transform: translate(-50%, -50%); color: var(--muted); z-index: 10; font-size: 1rem; }
  .error { color: var(--error); }
  .viewer-wrapper { flex: 1; overflow: hidden; }
  .viewer-wrapper.hidden { visibility: hidden; }
  :global(perspective-viewer) { --pivot-background: var(--bg); --plugin--background: var(--bg); }
</style>

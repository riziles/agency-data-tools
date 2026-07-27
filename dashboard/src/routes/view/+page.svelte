<script lang="ts">
  import { onMount } from 'svelte';
  import { browser } from '$app/environment';
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { getQueryResult } from '$lib/stores.svelte.ts';
  import { tableToIPC } from 'apache-arrow';

  let loading = $state(true);
  let error = $state('');
  let info = $state({ sql: '', elapsedMs: 0, rowCount: 0 });
  let viewerEl: HTMLElement;

  onMount(async () => {
    if (!browser) return;

    const result = getQueryResult();
    if (!result) {
      goto('/');
      return;
    }

    info = {
      sql: result.sql,
      elapsedMs: result.elapsedMs,
      rowCount: result.table.numRows,
    };

    try {
      // Load Perspective dynamically
      await import('@finos/perspective-viewer');
      await import('@finos/perspective-viewer-datagrid');
      await import('@finos/perspective-viewer-d3fc');

      const ipcData = tableToIPC(result.table);
      const buffer = ipcData.buffer.slice(
        ipcData.byteOffset,
        ipcData.byteOffset + ipcData.byteLength
      );

      const viewer = viewerEl as any;
      const worker = viewer._perspective_worker;
      if (!worker) throw new Error('Perspective worker not available');

      const table = await worker.table(buffer);

      // Get column list from the schema
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
    <div class="loading">Loading Perspective viewer...</div>
  {:else if error}
    <div class="error">{error}</div>
  {:else}
    <div class="viewer-wrapper">
      <perspective-viewer
        bind:this={viewerEl}
        style="height: calc(100vh - 80px); width: 100%;"
      ></perspective-viewer>
    </div>
  {/if}
</div>

<style>
  .viewer-page {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: var(--bg);
  }

  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.75rem 1.5rem;
    background: var(--surface);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .back-link {
    color: var(--primary);
    text-decoration: none;
    font-size: 0.9rem;
  }

  .back-link:hover {
    text-decoration: underline;
  }

  .meta {
    display: flex;
    gap: 1rem;
    font-size: 0.85rem;
    color: var(--muted);
  }

  .row-count {
    color: var(--text);
    font-weight: 600;
  }

  .loading, .error {
    display: flex;
    align-items: center;
    justify-content: center;
    flex: 1;
    font-size: 1rem;
    color: var(--muted);
  }

  .error {
    color: var(--error);
  }

  .viewer-wrapper {
    flex: 1;
    overflow: hidden;
  }

  :global(perspective-viewer) {
    --pivot-background: var(--bg);
    --plugin--background: var(--bg);
  }
</style>

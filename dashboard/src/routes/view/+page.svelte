<script lang="ts">
  import { onMount } from 'svelte';
  import { browser } from '$app/environment';
  import { goto } from '$app/navigation';
  import { getQueryResult } from '$lib/stores.svelte';

  let loading = $state(true);
  let info = $state({ sql: '', elapsedMs: 0, rowCount: 0 });
  let columns = $state<string[]>([]);
  let rows = $state<any[][]>([]);

  onMount(() => {
    if (!browser) return;

    const result = getQueryResult();
    if (!result) { goto('/'); return; }

    info = { sql: result.sql, elapsedMs: result.elapsedMs, rowCount: result.table.numRows };

    const fields = result.table.schema.fields;
    columns = fields.map((f: any) => f.name);

    const numRows = result.table.numRows;
    const rowData: any[][] = [];
    for (let i = 0; i < numRows; i++) {
      const row: any[] = [];
      for (const field of fields) {
        row.push(result.table.getChild(field.name)?.get(i));
      }
      rowData.push(row);
    }
    rows = rowData;
    loading = false;
  });

  function formatCell(value: any): string {
    if (value === null || value === undefined) return '—';
    if (typeof value === 'bigint') return value.toString();
    if (typeof value === 'number') {
      if (Number.isInteger(value)) return value.toLocaleString();
      return value.toFixed(4);
    }
    return String(value);
  }
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
    <div class="loading">Loading results...</div>
  {:else}
    <div class="table-wrapper">
      <table>
        <thead>
          <tr>
            {#each columns as col}
              <th>{col}</th>
            {/each}
          </tr>
        </thead>
        <tbody>
          {#each rows as row}
            <tr>
              {#each row as cell}
                <td>{formatCell(cell)}</td>
              {/each}
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
</div>

<style>
  .viewer-page { display: flex; flex-direction: column; height: 100vh; background: var(--bg); }
  header { display: flex; justify-content: space-between; align-items: center; padding: 0.75rem 1.5rem; background: var(--surface); border-bottom: 1px solid var(--border); flex-shrink: 0; }
  .back-link { color: var(--primary); text-decoration: none; font-size: 0.9rem; }
  .back-link:hover { text-decoration: underline; }
  .meta { display: flex; gap: 1rem; font-size: 0.85rem; color: var(--muted); }
  .row-count { color: var(--text); font-weight: 600; }
  .loading { display: flex; align-items: center; justify-content: center; flex: 1; color: var(--muted); }

  .table-wrapper { flex: 1; overflow: auto; }
  table { width: 100%; border-collapse: collapse; font-size: 0.85rem; }
  thead { position: sticky; top: 0; z-index: 1; }
  th { background: var(--surface); color: var(--muted); font-weight: 600; text-align: left; padding: 0.5rem 0.75rem; border-bottom: 2px solid var(--border); white-space: nowrap; }
  td { padding: 0.4rem 0.75rem; border-bottom: 1px solid rgba(51, 51, 51, 0.5); white-space: nowrap; }
  tr:hover td { background: rgba(233, 69, 96, 0.05); }
</style>

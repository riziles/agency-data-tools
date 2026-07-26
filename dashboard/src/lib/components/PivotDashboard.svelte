<script lang="ts">
  import { onMount } from 'svelte';
  import { browser } from '$app/environment';
  import type { FlightClient } from '$lib/flight-client';
  import { tableToIPC } from 'apache-arrow';

  let { client }: { client: FlightClient } = $props();

  // ── State ──
  let loading = $state(false);
  let error = $state('');
  let sqlPreview = $state('');
  let rowCount = $state(0);
  let elapsedMs = $state(0);
  let perspectiveReady = $state(false);

  // SQL builder state
  let selectedColumns = $state<string[]>(['property_state', 'original_upb']);
  let groupBy = $state('property_state');
  let aggregate = $state('AVG');

  // All available columns (subset for UI)
  const columns = [
    'loan_id', 'property_state', 'property_type', 'original_upb',
    'original_interest_rate', 'original_loan_term',
    'borrower_credit_score_at_origination',
    'co_borrower_credit_score_at_origination',
    'first_time_home_buyer_indicator', 'loan_purpose',
    'origination_date', 'maturity_date', 'number_of_borrowers',
  ];

  const aggregates = ['COUNT', 'SUM', 'AVG', 'MIN', 'MAX'];
  const groupByOptions = [
    'property_state', 'property_type', 'first_time_home_buyer_indicator',
    'loan_purpose', 'origination_date',
  ];

  let viewerEl: HTMLElement;

  // ── Build SQL ──
  function buildSQL(): string {
    const selectParts = selectedColumns.map((col) => {
      if (col === groupBy) return col;
      // Numeric columns get aggregated, others use COUNT
      const numericCols = ['original_upb', 'original_interest_rate', 'original_loan_term',
        'borrower_credit_score_at_origination', 'co_borrower_credit_score_at_origination'];
      if (numericCols.includes(col)) {
        return `${aggregate}(${col}) AS ${col}`;
      }
      return `COUNT(${col}) AS ${col}`;
    });
    return `SELECT ${selectParts.join(', ')}
FROM ducklake.main.loans
GROUP BY ${groupBy}
LIMIT 10000`;
  }

  async function runQuery() {
    const sql = buildSQL();
    sqlPreview = sql;
    loading = true;
    error = '';

    try {
      const result = await client.query(sql);
      rowCount = result.rowCount;
      elapsedMs = result.elapsedMs;

      // Feed Arrow table to Perspective
      const worker = (viewerEl as any)._perspective_worker;
      if (!worker) throw new Error('Perspective worker not ready');

      const tableName = `query_${Date.now()}`;
      const ipcData = tableToIPC(result.table);
      const table = await worker.table(ipcData.buffer.slice(
        ipcData.byteOffset, ipcData.byteOffset + ipcData.byteLength
      ));

      // Load into viewer
      await (viewerEl as any).load(table);
      await (viewerEl as any).restore({
        columns: selectedColumns,
        group_by: [groupBy],
        sort: [[groupBy, 'asc']],
      });
    } catch (e: any) {
      error = e.message || String(e);
    } finally {
      loading = false;
    }
  }

  onMount(async () => {
    // Load Perspective in browser only
    if (!browser) return;
    await import('@finos/perspective-viewer');
    await import('@finos/perspective-viewer-datagrid');
    await import('@finos/perspective-viewer-d3fc');
    perspectiveReady = true;

    const viewer = viewerEl as any;
    if (viewer.restore) {
      viewer.restore({
        plugin: 'Datagrid',
        settings: false,
        theme: 'Pro Dark',
      });
    }
  });
</script>

<div class="pivot-dashboard">
  <!-- Controls -->
  <div class="controls">
    <div class="control-group">
      <label>
        Group By
        <select bind:value={groupBy}>
          {#each groupByOptions as opt}
            <option value={opt}>{opt}</option>
          {/each}
        </select>
      </label>
    </div>

    <div class="control-group">
      <label>
        Aggregate
        <select bind:value={aggregate}>
          {#each aggregates as agg}
            <option value={agg}>{agg}</option>
          {/each}
        </select>
      </label>
    </div>

    <div class="control-group columns-group">
      <span class="label-text">Columns</span>
      <div class="columns-grid">
        {#each columns as col}
          <label class="checkbox-label">
            <input
              type="checkbox"
              checked={selectedColumns.includes(col)}
              onchange={(e) => {
                const checked = (e.target as HTMLInputElement).checked;
                selectedColumns = checked
                  ? [...selectedColumns, col]
                  : selectedColumns.filter((c) => c !== col);
              }}
            />
            {col}
          </label>
        {/each}
      </div>
    </div>

    <button class="run-btn" onclick={runQuery} disabled={loading}>
      {loading ? 'Running...' : 'Run Query'}
    </button>
  </div>

  <!-- Status -->
  {#if sqlPreview}
    <pre class="sql-preview">{sqlPreview}</pre>
  {/if}
  {#if error}
    <div class="error">{error}</div>
  {/if}
  {#if rowCount > 0}
    <div class="status-bar">
      {rowCount.toLocaleString()} rows in {elapsedMs}ms
    </div>
  {/if}

  <!-- Perspective Viewer -->
  <div class="viewer-wrapper">
    <perspective-viewer
      bind:this={viewerEl}
      style="height: 500px; width: 100%;"
    ></perspective-viewer>
  </div>
</div>

<style>
  .pivot-dashboard {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .controls {
    display: flex;
    flex-wrap: wrap;
    gap: 1rem;
    align-items: flex-end;
    padding: 1rem;
    background: var(--surface, #16213e);
    border-radius: 8px;
    border: 1px solid var(--border, #333);
  }

  .control-group {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  label, .label-text {
    font-size: 0.8rem;
    color: var(--muted, #888);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  label select {
    display: block;
    margin-top: 0.25rem;
  }

  select {
    background: var(--bg, #1a1a2e);
    color: var(--text, #eaeaea);
    border: 1px solid var(--border, #333);
    border-radius: 6px;
    padding: 0.5rem 0.75rem;
    font-size: 0.9rem;
    min-width: 200px;
  }

  .columns-group {
    flex: 1;
    min-width: 300px;
  }

  .columns-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 0.25rem;
    max-height: 200px;
    overflow-y: auto;
  }

  .checkbox-label {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    font-size: 0.85rem;
    color: var(--text, #eaeaea);
    cursor: pointer;
    padding: 0.15rem 0;
  }

  .checkbox-label input {
    accent-color: var(--primary, #e94560);
  }

  .run-btn {
    background: var(--primary, #e94560);
    color: white;
    border: none;
    border-radius: 6px;
    padding: 0.6rem 1.5rem;
    font-size: 0.95rem;
    font-weight: 600;
    cursor: pointer;
    transition: opacity 0.15s;
  }

  .run-btn:hover:not(:disabled) {
    opacity: 0.85;
  }

  .run-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .sql-preview {
    background: var(--surface, #16213e);
    color: var(--muted, #888);
    padding: 0.75rem 1rem;
    border-radius: 6px;
    font-size: 0.85rem;
    margin: 0;
    border: 1px solid var(--border, #333);
  }

  .error {
    background: rgba(233, 69, 96, 0.1);
    color: var(--error, #f44336);
    padding: 0.75rem 1rem;
    border-radius: 6px;
    font-size: 0.9rem;
    border: 1px solid var(--error, #f44336);
  }

  .status-bar {
    font-size: 0.85rem;
    color: var(--muted, #888);
  }

  .viewer-wrapper {
    border: 1px solid var(--border, #333);
    border-radius: 8px;
    overflow: hidden;
  }

  /* Perspective dark theme overrides */
  :global(perspective-viewer) {
    --pivot-background: var(--bg, #1a1a2e);
    --plugin--background: var(--bg, #1a1a2e);
  }
</style>

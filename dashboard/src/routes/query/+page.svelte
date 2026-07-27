<script lang="ts">
  import '../../app.css';
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { FlightClient, validateQuery } from '$lib/flight-client';
  import { setQueryResult } from '$lib/stores.svelte';

  let client = $state<FlightClient | null>(null);
  let connecting = $state(true);
  let connectionError = $state('');

  let sql = $state('');
  let running = $state(false);
  let queryError = $state('');

  onMount(async () => {
    try {
      const params = new URLSearchParams(window.location.search);
      const token = params.get('token') || 'demo';
      client = new FlightClient(token);
      await client.init();
    } catch (e: any) {
      connectionError = e.message || String(e);
    } finally {
      connecting = false;
    }
  });

  async function run() {
    if (!client || !sql.trim()) return;
    const validation = validateQuery(sql);
    if (validation) {
      queryError = validation;
      return;
    }
    running = true;
    queryError = '';
    try {
      const result = await client.query(sql);
      setQueryResult(result.table, sql, result.elapsedMs);
      await goto('/view');
    } catch (e: any) {
      queryError = e.message || String(e);
    } finally {
      running = false;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
      e.preventDefault();
      run();
    }
  }
</script>

<div class="app">
  <header>
    <a href="/" class="home-link">← Home</a>
    <div>
      <h1>Query Editor</h1>
      <span class="subtitle">751M rows · Flight SQL · Ctrl+Enter to run</span>
    </div>
    <div></div>
  </header>

  {#if connecting}
    <div class="status">Connecting to Flight SQL server...</div>
  {:else if connectionError}
    <div class="error">Connection failed: {connectionError}</div>
  {:else}
    <div class="editor">
      <textarea
        bind:value={sql}
        onkeydown={handleKeydown}
        placeholder="SELECT ... FROM ducklake.main.loans ..."
        disabled={running}
        spellcheck="false"
      ></textarea>
      <div class="editor-footer">
        <button class="run-btn" onclick={run} disabled={running || !sql.trim()}>
          {running ? 'Running...' : 'Run Query'}
        </button>
      </div>
      {#if queryError}
        <div class="error-msg">{queryError}</div>
      {/if}
    </div>

    <div class="examples">
      <h2>Examples</h2>
      <ul>
        <li><button onclick={() => sql = "SELECT count(*) AS total FROM ducklake.main.loans"}>Total row count</button></li>
        <li><button onclick={() => sql = "SELECT property_state, count(*) AS loans, round(avg(original_upb), 0) AS avg_upb\nFROM ducklake.main.loans\nGROUP BY property_state\nORDER BY loans DESC\nLIMIT 10"}>Loans by state</button></li>
        <li><button onclick={() => sql = "SELECT \n  substr(origination_date, 1, 4) || '-' || substr(origination_date, 5, 2) AS orig_month,\n  count(*) AS loans,\n  round(sum(original_upb) / 1e9, 2) AS total_upb_billions,\n  round(avg(original_upb), 0) AS avg_upb\nFROM ducklake.main.loans\nWHERE origination_date IS NOT NULL\nGROUP BY 1\nORDER BY 1"}>UPB by origination month</button></li>
        <li><button onclick={() => sql = "SELECT \n  round(borrower_credit_score_at_origination / 50) * 50 AS score_bucket,\n  count(*) AS loans,\n  round(avg(original_upb), 0) AS avg_upb\nFROM ducklake.main.loans\nWHERE borrower_credit_score_at_origination > 0\nGROUP BY 1\nORDER BY 1"}>Credit score distribution</button></li>
      </ul>
    </div>
  {/if}
</div>

<style>
  .app { max-width: 900px; margin: 0 auto; padding: 2rem; }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 2rem;
    gap: 1rem;
  }

  header h1 { font-size: 1.3rem; margin-bottom: 0.25rem; }

  .home-link {
    color: var(--muted);
    text-decoration: none;
    font-size: 0.9rem;
  }
  .home-link:hover { color: var(--text); }

  .subtitle { color: var(--muted); font-size: 0.85rem; }

  .status { color: var(--muted); padding: 2rem; text-align: center; }

  .error {
    background: rgba(233, 69, 96, 0.1);
    color: var(--error);
    padding: 1rem;
    border-radius: 8px;
    border: 1px solid var(--error);
  }

  .editor {
    background: var(--surface);
    border-radius: 8px;
    border: 1px solid var(--border);
    overflow: hidden;
  }

  textarea {
    width: 100%;
    min-height: 200px;
    background: transparent;
    color: var(--text);
    border: none;
    padding: 1rem;
    font-family: 'JetBrains Mono', 'Fira Code', monospace;
    font-size: 0.9rem;
    line-height: 1.6;
    resize: vertical;
    outline: none;
  }

  .editor-footer {
    display: flex;
    justify-content: flex-end;
    padding: 0.75rem 1rem;
    background: rgba(0, 0, 0, 0.2);
    border-top: 1px solid var(--border);
  }

  .run-btn {
    background: var(--primary);
    color: white;
    border: none;
    border-radius: 6px;
    padding: 0.5rem 1.5rem;
    font-size: 0.9rem;
    font-weight: 600;
    cursor: pointer;
    transition: opacity 0.15s;
  }
  .run-btn:hover:not(:disabled) { opacity: 0.85; }
  .run-btn:disabled { opacity: 0.4; cursor: not-allowed; }

  .error-msg {
    color: var(--error);
    padding: 0.75rem 1rem;
    font-size: 0.85rem;
    background: rgba(233, 69, 96, 0.08);
    border-top: 1px solid rgba(233, 69, 96, 0.2);
  }

  .examples { margin-top: 2rem; }
  .examples h2 { font-size: 1rem; margin-bottom: 0.75rem; }
  .examples ul { list-style: none; display: flex; flex-wrap: wrap; gap: 0.5rem; }
  .examples button {
    background: var(--surface);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 0.4rem 0.8rem;
    font-size: 0.8rem;
    cursor: pointer;
    transition: border-color 0.15s;
    white-space: nowrap;
  }
  .examples button:hover { border-color: var(--primary); }
</style>

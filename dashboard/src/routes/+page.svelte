<script lang="ts">
  import '../app.css';
  import { onMount } from 'svelte';
  import { FlightClient } from '$lib/flight-client';
  import PivotDashboard from '$lib/components/PivotDashboard.svelte';

  let client = $state<FlightClient | null>(null);
  let connecting = $state(true);
  let error = $state('');

  onMount(async () => {
    try {
      const params = new URLSearchParams(window.location.search);
      const token = params.get('token') || 'demo';
      client = new FlightClient(token);
      await client.init();
    } catch (e: any) {
      error = e.message || String(e);
    } finally {
      connecting = false;
    }
  });
</script>

<div class="app">
  <header>
    <h1>🏠 Fannie Mae Loan Performance</h1>
    <span class="subtitle">751M rows · 32 quarters · DataFusion Flight SQL</span>
  </header>

  {#if connecting}
    <div class="status">Connecting to Flight SQL server...</div>
  {:else if error}
    <div class="error">Connection failed: {error}</div>
  {:else if client}
    <PivotDashboard {client} />
  {/if}
</div>

<style>
  .app {
    max-width: 1200px;
    margin: 0 auto;
    padding: 2rem;
  }

  header {
    margin-bottom: 2rem;
  }

  .subtitle {
    color: var(--muted);
    font-size: 0.9rem;
  }

  .status {
    color: var(--muted);
    padding: 2rem;
    text-align: center;
  }

  .error {
    background: rgba(233, 69, 96, 0.1);
    color: var(--error);
    padding: 1rem;
    border-radius: 8px;
    border: 1px solid var(--error);
  }
</style>

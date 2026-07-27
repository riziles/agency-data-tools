<script lang="ts">
  import '../app.css';
</script>

<svelte:head>
  <title>Fannie Mae Loan Performance — DataFusion Analytics</title>
</svelte:head>

<div class="landing">
  <header>
    <h1>🏠 Fannie Mae Loan Performance</h1>
    <p class="subtitle">751 million rows · 32 quarters (2018–2025) · Apache DataFusion + DuckLake</p>
  </header>

  <div class="cards">
    <a href="/query" class="card">
      <span class="card-icon">⚡</span>
      <h2>Query Editor</h2>
      <p>Write SQL queries against the full dataset. Results open in a Perspective pivot table with drag-and-drop dimensions, charts, and exports.</p>
      <span class="card-link">Open →</span>
    </a>

    <a href="/docs" class="card">
      <span class="card-icon">📖</span>
      <h2>API Docs</h2>
      <p>Flight SQL endpoint reference, connection examples, schema documentation, and query guard-rails.</p>
      <span class="card-link">View →</span>
    </a>
  </div>

  <section class="about">
    <h2>About</h2>
    <p>
      This is a local-first analytics pipeline for Fannie Mae single-family loan performance data.
      The full dataset — 751 million rows across 32 quarters from 2018 to 2025 — is stored in a
      <strong>DuckLake</strong> catalog backed by Apache Parquet files and queried through an
      <strong>Apache DataFusion Flight SQL</strong> server.
    </p>
    <p>
      The browser connects via gRPC-web. Queries aggregate server-side over the full dataset
      (typically ~500ms for <code>count(*)</code>) and return Apache Arrow record batches.
      Results are visualized using Perspective.js with pivot tables, charts, and filtering.
    </p>

    <h3>Stack</h3>
    <div class="stack">
      <span>DataFusion 54</span>
      <span>DuckLake 0.5</span>
      <span>Arrow Flight SQL</span>
      <span>Perspective 3.8</span>
      <span>Svelte 5</span>
      <span>Docker</span>
    </div>
  </section>

  <footer>
    <a href="https://github.com/your-org/agency-data-tools">GitHub</a>
    <span>·</span>
    <a href="https://datafusion.apache.org">Apache DataFusion</a>
    <span>·</span>
    <a href="https://perspective.finos.org">Perspective</a>
  </footer>
</div>

<style>
  .landing {
    max-width: 800px;
    margin: 0 auto;
    padding: 3rem 2rem;
  }

  header {
    text-align: center;
    margin-bottom: 3rem;
  }

  header h1 {
    font-size: 2rem;
    margin-bottom: 0.5rem;
  }

  .subtitle {
    color: var(--muted);
    font-size: 1rem;
  }

  .cards {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1.5rem;
    margin-bottom: 3rem;
  }

  .card {
    display: flex;
    flex-direction: column;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 2rem;
    text-decoration: none;
    color: var(--text);
    transition: border-color 0.2s, transform 0.15s;
  }

  .card:hover {
    border-color: var(--primary);
    transform: translateY(-2px);
  }

  .card-icon {
    font-size: 2rem;
    margin-bottom: 0.75rem;
  }

  .card h2 {
    font-size: 1.2rem;
    margin-bottom: 0.5rem;
    color: var(--text);
  }

  .card p {
    color: var(--muted);
    font-size: 0.9rem;
    line-height: 1.5;
    flex: 1;
  }

  .card-link {
    color: var(--primary);
    font-weight: 600;
    font-size: 0.85rem;
    margin-top: 1rem;
  }

  .about {
    margin-bottom: 2rem;
  }

  .about h2 {
    font-size: 1.1rem;
    margin-bottom: 0.75rem;
  }

  .about h3 {
    font-size: 1rem;
    margin: 1.5rem 0 0.75rem;
  }

  .about p {
    color: var(--muted);
    font-size: 0.9rem;
    line-height: 1.6;
    margin-bottom: 0.75rem;
  }

  .about code {
    background: var(--surface);
    padding: 0.15em 0.4em;
    border-radius: 4px;
    font-size: 0.85em;
  }

  .stack {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
  }

  .stack span {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 0.3rem 0.8rem;
    font-size: 0.8rem;
    color: var(--muted);
  }

  footer {
    text-align: center;
    padding-top: 2rem;
    border-top: 1px solid var(--border);
    color: var(--muted);
    font-size: 0.8rem;
  }

  footer a {
    color: var(--muted);
    text-decoration: none;
  }

  footer a:hover {
    color: var(--text);
  }
</style>

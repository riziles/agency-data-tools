<script lang="ts">
  import '../../app.css';
</script>

<svelte:head>
  <title>API Docs — Fannie Mae Flight SQL</title>
</svelte:head>

<div class="docs">
  <header>
    <a href="/" class="home-link">← Home</a>
    <h1>Flight SQL API Docs</h1>
  </header>

  <section>
    <h2>Endpoint</h2>
    <table>
      <tbody>
        <tr><th>Protocol</th><td>Apache Arrow Flight SQL (gRPC)</td></tr>
        <tr><th>Transport</th><td>gRPC-web (HTTP/1.1) via Node proxy on same origin</td></tr>
        <tr><th>Backend</th><td>Flight SQL :50051 (DataFusion 54 + DuckLake 0.5)</td></tr>
        <tr><th>Auth</th><td>Password gate (cookie or <code>x-auth-token</code> header)</td></tr>
      </tbody>
    </table>
  </section>

  <section>
    <h2>Connection</h2>

    <h3>Browser (gRPC-web)</h3>
    <pre><code>import &#123; connect &#125; from '@sparrowflight/js';

const client = await connect(&#123;
  endpoint: '/',  // same origin — works locally and through Cloudflare Tunnel
  headers: &#123; 'x-auth-token': 'demo' &#125;,
&#125;);

const &#123; table &#125; = await client.query(
  "SELECT count(*) FROM ducklake.main.loans"
);</code></pre>

    <h3>gRPC (native)</h3>
    <pre><code># Direct to Flight SQL (bypasses auth proxy)
grpcurl -plaintext \
  -d '&#123;"query":"SELECT count(*) FROM ducklake.main.loans"&#125;' \
  localhost:50051 \
  arrow.flight.protocol.FlightService/GetFlightInfo</code></pre>

    <h3>Python</h3>
    <pre><code># Direct Flight SQL (bypasses auth proxy)
from flightsql import FlightSQLClient

client = FlightSQLClient(
    host='localhost',
    port=50051,
    insecure=True,
)
info = client.execute("SELECT count(*) FROM ducklake.main.loans")
table = client.do_get(info.endpoints[0].ticket)
print(table.read_all())</code></pre>
  </section>

  <section>
    <h2>Schema</h2>
    <p>
      Table: <code>ducklake.main.loans</code> — 113 columns, ~751M rows, 32 quarters.
    </p>

    <h3>Key Columns</h3>
    <table>
      <thead>
        <tr><th>Column</th><th>Type</th><th>Description</th></tr>
      </thead>
      <tbody>
        <tr><td><code>loan_id</code></td><td>Utf8</td><td>Unique loan identifier</td></tr>
        <tr><td><code>property_state</code></td><td>Utf8</td><td>2-letter state abbreviation</td></tr>
        <tr><td><code>property_type</code></td><td>Utf8</td><td>CO, PU, SF, MH, CP</td></tr>
        <tr><td><code>original_upb</code></td><td>Float64</td><td>Original unpaid principal balance</td></tr>
        <tr><td><code>original_interest_rate</code></td><td>Float64</td><td>Original interest rate (%)</td></tr>
        <tr><td><code>original_loan_term</code></td><td>Float64</td><td>Original loan term (months)</td></tr>
        <tr><td><code>borrower_credit_score_at_origination</code></td><td>Int32</td><td>Primary borrower credit score</td></tr>
        <tr><td><code>co_borrower_credit_score_at_origination</code></td><td>Int32</td><td>Co-borrower credit score (nullable)</td></tr>
        <tr><td><code>first_time_home_buyer_indicator</code></td><td>Utf8</td><td>Y, N, 7, 9</td></tr>
        <tr><td><code>loan_purpose</code></td><td>Utf8</td><td>P, R, C</td></tr>
        <tr><td><code>origination_date</code></td><td>Utf8</td><td>Loan origination date (YYYYMM)</td></tr>
        <tr><td><code>maturity_date</code></td><td>Utf8</td><td>Loan maturity date (YYYYMM)</td></tr>
        <tr><td><code>number_of_borrowers</code></td><td>Float64</td><td>Number of borrowers</td></tr>
      </tbody>
    </table>

    <p class="note">
      Full 113-column schema: <a href="http://capitalmarkets.fanniemae.com/sites/capmrkt/files/2023-06/crt-file-layout-and-glossary.pdf">CRT File Layout and Glossary (PDF)</a>
    </p>
  </section>

  <section>
    <h2>Query Guard-Rails</h2>
    <table>
      <tbody>
        <tr><th>Rule</th><th>Details</th></tr>
        <tr><td><code>SELECT</code> only</td><td>INSERT, UPDATE, DELETE are blocked</td></tr>
        <tr><td><code>GROUP BY</code> or <code>LIMIT</code> required</td><td>Prevent unbounded <code>SELECT *</code></td></tr>
        <tr><td>Aggregate exception</td><td>Queries with <code>COUNT</code>, <code>SUM</code>, <code>AVG</code>, <code>MIN</code>, <code>MAX</code> allowed without GROUP BY/LIMIT</td></tr>
        <tr><td><code>LIMIT</code> cap</td><td>Maximum 10,000 rows per query</td></tr>
      </tbody>
    </table>
  </section>

  <section>
    <h2>Dataset</h2>
    <table>
      <thead>
        <tr><th>Year</th><th style="text-align:right">Rows</th><th>Notes</th></tr>
      </thead>
      <tbody>
        <tr><td>2018</td><td style="text-align:right">78M</td><td>Pre-COVID baseline</td></tr>
        <tr><td>2019</td><td style="text-align:right">79M</td><td>Steady market</td></tr>
        <tr><td>2020</td><td style="text-align:right">235M</td><td>COVID refi boom</td></tr>
        <tr><td>2021</td><td style="text-align:right">236M</td><td>Peak refi era</td></tr>
        <tr><td>2022</td><td style="text-align:right">73M</td><td>Market cooling</td></tr>
        <tr><td>2023</td><td style="text-align:right">27M</td><td>Rate hikes</td></tr>
        <tr><td>2024</td><td style="text-align:right">17M</td><td>Continued decline</td></tr>
        <tr><td>2025</td><td style="text-align:right">6M</td><td>Low-volume market</td></tr>
        <tr class="total"><td><strong>Total</strong></td><td style="text-align:right"><strong>~751M</strong></td><td>32 quarters</td></tr>
      </tbody>
    </table>
  </section>

  <footer>
    <a href="/query">Query Editor →</a>
    <span>·</span>
    <a href="https://arrow.apache.org/docs/format/FlightSql.html">Arrow Flight SQL Spec</a>
    <span>·</span>
    <a href="https://datafusion.apache.org">Apache DataFusion</a>
  </footer>
</div>

<style>
  .docs { max-width: 800px; margin: 0 auto; padding: 2rem; }
  header { margin-bottom: 2rem; }
  header h1 { font-size: 1.5rem; margin-top: 0.5rem; }
  .home-link { color: var(--muted); text-decoration: none; font-size: 0.9rem; }
  .home-link:hover { color: var(--text); }
  section { margin-bottom: 2.5rem; }
  h2 { font-size: 1.15rem; margin-bottom: 1rem; padding-bottom: 0.5rem; border-bottom: 1px solid var(--border); }
  h3 { font-size: 1rem; margin: 1.5rem 0 0.75rem; color: var(--muted); }
  p { color: var(--muted); font-size: 0.9rem; line-height: 1.6; margin-bottom: 0.75rem; }
  code { background: var(--surface); padding: 0.15em 0.4em; border-radius: 4px; font-size: 0.85em; color: var(--text); }
  pre { background: var(--surface); border: 1px solid var(--border); border-radius: 8px; padding: 1rem; overflow-x: auto; font-size: 0.85rem; line-height: 1.5; margin-bottom: 1rem; }
  pre code { background: none; padding: 0; }
  table { width: 100%; border-collapse: collapse; font-size: 0.9rem; margin-bottom: 1rem; }
  th, td { text-align: left; padding: 0.5rem 0.75rem; border-bottom: 1px solid var(--border); }
  th { color: var(--muted); font-weight: 600; font-size: 0.8rem; text-transform: uppercase; letter-spacing: 0.05em; }
  td { color: var(--text); }
  .total td { border-bottom: none; padding-top: 0.75rem; }
  .note { font-size: 0.8rem; margin-top: 0.5rem; }
  .note a { color: var(--primary); }
  footer { text-align: center; padding-top: 2rem; border-top: 1px solid var(--border); color: var(--muted); font-size: 0.8rem; }
  footer a { color: var(--muted); text-decoration: none; }
  footer a:hover { color: var(--text); }
</style>

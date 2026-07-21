import { connect } from "@sparrowflight/js";

const R = (id) => document.getElementById(id);

let client = null;

// Start query on Ctrl+Enter
R("sql-input").addEventListener("keydown", (e) => {
  if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
    e.preventDefault();
    runQuery();
  }
});
R("run-btn").addEventListener("click", runQuery);

async function init() {
  R("load-status").textContent = "Connecting to Flight SQL server...";
  R("progress-fill").style.width = "20%";

  try {
    client = await connect({
      endpoint: window.location.origin,
    });
    R("progress-fill").style.width = "100%";
    R("load-status").textContent = "Connected to DataFusion Flight SQL";
  } catch (e) {
    R("load-status").textContent = "Connection failed: " + (e.message || e);
    R("progress-fill").style.background = "var(--error)";
    console.error(e);
    return;
  }

  setTimeout(() => {
    R("loading").classList.add("hidden");
    R("app").classList.remove("hidden");
  }, 300);
}

async function runQuery() {
  const sql = R("sql-input").value.trim();
  if (!sql || !client) return;

  R("error").classList.add("hidden");
  R("result-table-wrapper").classList.add("hidden");
  R("status").textContent = "Running...";
  R("status").className = "status loading";
  R("run-btn").disabled = true;

  try {
    const t0 = performance.now();
    const { table, stats } = await client.query(sql);
    const ms = (performance.now() - t0).toFixed(0);

    R("status").textContent =
      `${stats.rows?.toLocaleString() ?? table.numRows?.toLocaleString()} rows in ${ms}ms`;
    R("status").className = "status success";

    const rows = table.numRows ?? 0;
    if (rows === 0) {
      R("error").textContent = "No results.";
      R("error").classList.remove("hidden");
      return;
    }

    renderTable(table);
  } catch (e) {
    R("status").textContent = "Error";
    R("status").className = "status error";
    R("error").textContent = e.message || String(e);
    R("error").classList.remove("hidden");
  } finally {
    R("run-btn").disabled = false;
  }
}

function renderTable(table) {
  const keys = table.schema.fields.map((f) => f.name);

  const thead = document.createElement("thead");
  const hr = document.createElement("tr");
  keys.forEach((k) => {
    const th = document.createElement("th");
    th.textContent = k;
    hr.appendChild(th);
  });
  thead.appendChild(hr);

  const tbody = document.createElement("tbody");
  for (let i = 0; i < table.numRows; i++) {
    const tr = document.createElement("tr");
    keys.forEach((k) => {
      const td = document.createElement("td");
      const col = table.getChild(k);
      td.textContent = col ? String(col.get(i)) : "";
      tr.appendChild(td);
    });
    tbody.appendChild(tr);
  }

  R("result-table").innerHTML = "";
  R("result-table").appendChild(thead);
  R("result-table").appendChild(tbody);
  R("row-count").textContent = table.numRows.toLocaleString() + " rows";
  R("result-table-wrapper").classList.remove("hidden");
}

init();

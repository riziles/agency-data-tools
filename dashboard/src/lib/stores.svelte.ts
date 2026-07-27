import type { Table } from 'apache-arrow';

// In-memory store for passing Arrow query results between pages
let lastResult = $state<{ table: Table; sql: string; elapsedMs: number } | null>(null);

export function setQueryResult(table: Table, sql: string, elapsedMs: number) {
  lastResult = { table, sql, elapsedMs };
}

export function getQueryResult() {
  return lastResult;
}

export function clearQueryResult() {
  lastResult = null;
}

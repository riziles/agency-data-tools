import { connect } from '@sparrowflight/js';
import type { ArrowTable } from 'apache-arrow';

interface QueryResult {
  table: ArrowTable;
  rowCount: number;
  elapsedMs: number;
}

export class FlightClient {
  private client: Awaited<ReturnType<typeof connect>> | null = null;
  private token: string;

  constructor(token: string) {
    this.token = token;
  }

  async init(): Promise<void> {
    if (this.client) return;
    this.client = await connect({
      endpoint: window.location.origin,
      headers: this.token ? { 'x-auth-token': this.token } : {},
    });
  }

  async query(sql: string, signal?: AbortSignal): Promise<QueryResult> {
    if (!this.client) await this.init();

    const start = performance.now();
    const { table } = await this.client!.query(sql);
    const elapsedMs = Math.round(performance.now() - start);

    return {
      table,
      rowCount: table.numRows,
      elapsedMs,
    };
  }
}

/**
 * Validate a user SQL query against guard-rails.
 * Returns an error message string, or null if valid.
 */
export function validateQuery(sql: string): string | null {
  const upper = sql.toUpperCase().trim();

  if (!upper) return 'Query is empty';
  if (!upper.startsWith('SELECT')) return 'Only SELECT queries are allowed';

  // Require GROUP BY or LIMIT for non-trivial queries
  if (!upper.includes('GROUP BY') && !upper.includes('LIMIT')) {
    return 'Query must include GROUP BY or LIMIT clause';
  }

  // Enforce row limit
  const limitMatch = upper.match(/LIMIT\s+(\d+)/);
  if (limitMatch) {
    const limit = parseInt(limitMatch[1], 10);
    if (limit > 10_000) {
      return 'LIMIT cannot exceed 10,000 rows';
    }
  }

  return null;
}

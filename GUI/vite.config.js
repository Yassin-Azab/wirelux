// vite.config.js — complete rewrite with world-atlas static asset serving
import { defineConfig } from 'vite';
import path from 'path';
import fs from 'fs';
import { createRequire } from 'module';

const __dirname = path.dirname(new URL(import.meta.url).pathname);
const CONFIG_PATH = path.resolve(__dirname, '../wirelux/config.toml');

// ── DB path resolution ────────────────────────────────────────────────────────
function resolveDbPath() {
  try {
    const raw = fs.readFileSync(CONFIG_PATH, 'utf8').trim();
    // Handle bare path or TOML assignment: db = "./foo.db"
    const match = raw.match(/=\s*["']?([^"'\n#]+)["']?/);
    const rel   = (match ? match[1] : raw).trim();
    return path.resolve(path.dirname(CONFIG_PATH), rel);
  } catch {
    const dir = path.dirname(CONFIG_PATH);
    const db  = fs.readdirSync(dir).find(f => f.endsWith('.db') && !f.endsWith('-shm') && !f.endsWith('-wal'));
    if (db) return path.join(dir, db);
    throw new Error('Cannot find database — check config.toml');
  }
}

// ── SQLite API plugin ─────────────────────────────────────────────────────────
function sqliteApiPlugin() {
  const require = createRequire(import.meta.url);
  let db = null;

  function getDb() {
    if (db) return db;
    const dbPath = resolveDbPath();
    console.log(`[Wirelux] DB → ${dbPath}`);
    const Database = require('better-sqlite3');
    db = new Database(dbPath, { readonly: true, fileMustExist: true });
    db.pragma('journal_mode = WAL');
    db.pragma('cache_size = -65536');   // 64 MB page cache
    db.pragma('temp_store = MEMORY');
    return db;
  }

  function whereFromParams(params) {
    const conds = [];
    const vals  = [];

    if (params.direction === 'incoming') { conds.push('direction = 0'); }
    else if (params.direction === 'outgoing') { conds.push('direction = 1'); }

    if (params.protocol && params.protocol !== 'all') {
      const map = { tcp: 6, udp: 17, icmp: 1 };
      if (map[params.protocol] !== undefined) {
        conds.push('protocol = ?');
        vals.push(map[params.protocol]);
      } else if (params.protocol === 'other') {
        conds.push('protocol NOT IN (6,17,1)');
      }
    }

    if (params.timeRange && params.timeRange !== 'all') {
      // TAI ns: now = Unix ms * 1e6 + TAI offset (37s = 37e9 ns)
      const TAI_OFFSET = 37_000_000_000n;
      const now = BigInt(Date.now()) * 1_000_000n + TAI_OFFSET;
      const mins = parseInt(params.timeRange, 10);
      const cutoff = now - BigInt(mins) * 60_000_000_000n;
      conds.push('timestamp_ns >= ?');
      vals.push(cutoff.toString());
    }

    if (params.process?.trim()) {
      conds.push('comms LIKE ?');
      vals.push(`%${params.process.trim()}%`);
    }
    if (params.country?.trim()) {
      conds.push('country LIKE ?');
      vals.push(`%${params.country.trim()}%`);
    }

    return {
      where: conds.length ? `WHERE ${conds.join(' AND ')}` : '',
      vals,
    };
  }

  function nsToDate(ns) {
    if (!ns) return 'N/A';
    try {
      const TAI_OFFSET = 37_000_000_000n;
      const unix = (BigInt(ns) - TAI_OFFSET) / 1_000_000n;
      return new Date(Number(unix)).toLocaleString();
    } catch { return 'N/A'; }
  }

  const PROTO_MAP = { 6: 'TCP', 17: 'UDP', 1: 'ICMP' };

  return {
    name: 'wirelux-sqlite-api',
    configureServer(server) {
      server.middlewares.use('/api', (req, res, next) => {
        const url      = new URL(req.url, 'http://x');
        const endpoint = url.pathname.replace(/^\/+/, '');
        const params   = Object.fromEntries(url.searchParams);

        res.setHeader('Content-Type', 'application/json; charset=utf-8');
        res.setHeader('Cache-Control', 'no-store');

        try {
          const database = getDb();

          // ── /api/config ───────────────────────────────────────────────────
          if (endpoint === 'config') {
            return res.end(JSON.stringify({
              dbPath: resolveDbPath(),
              refreshInterval: 5000,
              theme: 'dark',
            }));
          }

          // ── /api/local-country ────────────────────────────────────────────
          if (endpoint === 'local-country') {
            // Use the country that appears as the "local" origin — Egypt in the DB
            const row = database.prepare(
              "SELECT country FROM events WHERE country = 'Egypt' LIMIT 1"
            ).get();
            // Fallback: most common non-null country by row count
            const fallback = database.prepare(
              "SELECT country, COUNT(*) as n FROM events WHERE country IS NOT NULL AND country != '' GROUP BY country ORDER BY n DESC LIMIT 1"
            ).get();
            return res.end(JSON.stringify({
              country: row?.country ?? fallback?.country ?? 'Unknown',
            }));
          }

          // ── /api/connections ──────────────────────────────────────────────
          if (endpoint === 'connections') {
            const { where, vals } = whereFromParams(params);
            // Exclude the local country from destination list and Unknown
            const exclude = where
              ? `${where} AND country NOT IN ('Egypt','Unknown','')`
              : "WHERE country NOT IN ('Egypt','Unknown','')";

            const sql = `
              SELECT
                country,
                SUM(size)                                         AS total_bytes,
                SUM(CASE WHEN direction=0 THEN size ELSE 0 END)  AS incoming_bytes,
                SUM(CASE WHEN direction=1 THEN size ELSE 0 END)  AS outgoing_bytes,
                COUNT(*)                                          AS packet_count,
                COUNT(DISTINCT comms)                             AS unique_processes,
                (
                  SELECT comms FROM events sub
                  WHERE sub.country = e.country
                  ${where ? 'AND ' + where.replace(/^WHERE\s+/, '') : ''}
                  GROUP BY comms ORDER BY SUM(size) DESC LIMIT 1
                ) AS top_process,
                MIN(timestamp_ns) AS first_packet_ns,
                MAX(timestamp_ns) AS last_packet_ns,
                GROUP_CONCAT(DISTINCT protocol) AS protocols
              FROM events e
              ${exclude}
              GROUP BY country
              ORDER BY total_bytes DESC
            `;

            const rows = database.prepare(sql).all(...vals, ...vals);

            const result = rows.map(r => ({
              country:          r.country,
              total_bytes:      r.total_bytes,
              incoming_bytes:   r.incoming_bytes,
              outgoing_bytes:   r.outgoing_bytes,
              packet_count:     r.packet_count,
              unique_processes: r.unique_processes,
              top_process:      r.top_process,
              first_packet:     nsToDate(r.first_packet_ns),
              last_packet:      nsToDate(r.last_packet_ns),
              protocols: (r.protocols || '').split(',')
                .map(p => PROTO_MAP[+p] || `Proto ${p}`)
                .filter((v, i, a) => a.indexOf(v) === i)
                .join(', '),
            }));

            return res.end(JSON.stringify(result));
          }

          // ── /api/processes ────────────────────────────────────────────────
          if (endpoint === 'processes') {
            const { where, vals } = whereFromParams(params);
            const sql = `
              SELECT comms AS name, SUM(size) AS total_bytes
              FROM events
              ${where}
              GROUP BY comms
              ORDER BY total_bytes DESC
            `;
            const rows = database.prepare(sql).all(...vals);
            const grand = rows.reduce((s, r) => s + r.total_bytes, 0);
            return res.end(JSON.stringify({
              processes: rows.map(r => ({
                name:        r.name,
                total_bytes: r.total_bytes,
                percentage:  grand > 0 ? +(r.total_bytes / grand * 100).toFixed(2) : 0,
              })),
              total_bytes: grand,
            }));
          }

          next();
        } catch (err) {
          console.error('[Wirelux API]', err.message);
          res.statusCode = 500;
          res.end(JSON.stringify({ error: err.message }));
        }
      });
    },
  };
}

// ── world-atlas static serving ────────────────────────────────────────────────
function worldAtlasPlugin() {
  return {
    name: 'world-atlas-static',
    configureServer(server) {
      server.middlewares.use('/world-atlas', (req, res, next) => {
        const filePath = path.join(
          __dirname, 'node_modules', 'world-atlas',
          req.url.replace(/^\//, '')
        );
        if (fs.existsSync(filePath)) {
          res.setHeader('Content-Type', 'application/json');
          res.setHeader('Cache-Control', 'public, max-age=86400');
          fs.createReadStream(filePath).pipe(res);
        } else {
          next();
        }
      });
    },
  };
}

export default defineConfig({
  root: __dirname,
  plugins: [sqliteApiPlugin(), worldAtlasPlugin()],
  optimizeDeps: {
    exclude: ['better-sqlite3'],
  },
  server: {
    port: 5173,
    open: false,
  },
});

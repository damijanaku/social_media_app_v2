import dotenv from "dotenv";
import pg from "pg";
import readline from "readline";

dotenv.config();
const { Client } = pg;

const DEFAULT_TABLE_ORDER = ["comments", "likes", "follows", "posts", "users"];
const DATE_COLUMN = "created_at";

function parseArgs() {
  const args = {};
  for (const arg of process.argv.slice(2)) {
    if (arg === "--dry-run") {
      args.dryRun = true;
      continue;
    }
    const match = arg.match(/^--([^=]+)=(.*)$/);
    if (match) args[match[1]] = match[2];
  }
  return args;
}

function confirm(question) {
  const rl = readline.createInterface({ input: process.stdin, output: process.stdout });
  return new Promise((resolve) => {
    rl.question(question, (answer) => {
      rl.close();
      resolve(answer.trim().toLowerCase() === "yes");
    });
  });
}

async function countOld(client, table, cutoffDate) {
  const result = await client.query(
    `SELECT COUNT(*) FROM "${table}" WHERE "${DATE_COLUMN}" > $1`,
    [cutoffDate]
  );
  return parseInt(result.rows[0].count, 10);
}

async function deleteOld(client, table, cutoffDate) {
  const result = await client.query(
    `DELETE FROM "${table}" WHERE "${DATE_COLUMN}" > $1`,
    [cutoffDate]
  );
  return result.rowCount;
}

async function main() {
  const { after, dryRun, tables } = parseArgs();

  if (!after) {
    console.error(
      'Usage: node cleanup.js --after=YYYY-MM-DD [--dry-run] [--tables=Comment,Like,...]'
    );
    process.exit(1);
  }

  const cutoffDate = new Date(after);
  if (isNaN(cutoffDate.getTime())) {
    console.error(`Invalid date: "${after}". Use format YYYY-MM-DD.`);
    process.exit(1);
  }

  const requested = tables ? tables.split(",").map((t) => t.trim()) : null;
  const tableOrder = requested
    ? DEFAULT_TABLE_ORDER.filter((t) => requested.includes(t))
    : DEFAULT_TABLE_ORDER;

  if (tableOrder.length === 0) {
    console.error("No matching tables to process. Check your --tables list.");
    process.exit(1);
  }

  const client = new Client({
    connectionString: process.env.DATABASE_URL,
  });

  try {
    await client.connect();

    console.log(`Counting rows newer than ${after}...\n`);
    const counts = {};
    for (const table of tableOrder) {
      counts[table] = await countOld(client, table, cutoffDate);
      console.log(`  ${table}: ${counts[table]} row(s)`);
    }

    const total = Object.values(counts).reduce((a, b) => a + b, 0);
    if (total === 0) {
      console.log("\nNothing to delete.");
      return;
    }

    if (dryRun) {
      console.log("\nDry run only — no rows deleted.");
      return;
    }

    const confirmed = await confirm(
      `\nThis will permanently delete ${total} row(s) across ${tableOrder.length} table(s). Type "yes" to proceed: `
    );
    if (!confirmed) {
      console.log("Aborted. No rows deleted.");
      return;
    }

    console.log("\nDeleting...\n");
    for (const table of tableOrder) {
      if (counts[table] === 0) {
        console.log(`  ${table}: skipped `);
        continue;
      }
      try {
        const deleted = await deleteOld(client, table, cutoffDate);
        console.log(`  ${table}: deleted ${deleted} row(s)`);
      } catch (err) {
        console.error(`  ${table}: FAILED — ${err.message}`);
      }
    }
  } catch (err) {
    console.error("Error:", err.message);
    process.exit(1);
  } finally {
    await client.end();
  }
}

main();
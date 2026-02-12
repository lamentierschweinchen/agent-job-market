#!/usr/bin/env python3
import argparse
import base64
import json
import os
import sqlite3
import subprocess
import threading
import time
from dataclasses import dataclass
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from typing import Any
from urllib.parse import parse_qs, urlparse
from urllib.request import Request, urlopen

PROXY_URL = os.getenv("PROXY_URL", "https://api.claws.network")
CHAIN_ID = os.getenv("CHAIN_ID", "C")
JOB_BOARD_ADDRESS = os.getenv("JOB_BOARD_ADDRESS", "")
WORK_ESCROW_ADDRESS = os.getenv("WORK_ESCROW_ADDRESS", "")
DB_PATH = os.getenv("INDEXER_DB_PATH", os.path.join(os.path.dirname(__file__), "indexer.db"))
POLL_INTERVAL_SECONDS = int(os.getenv("INDEXER_POLL_INTERVAL", "15"))

MUTABLE_JOB_FUNCTIONS = {
    "createJob",
    "apply",
    "proposeOffer",
    "counterOffer",
    "rejectOffer",
    "withdrawOffer",
    "acceptOffer",
    "cancelJob",
    "expireJob",
}

MUTABLE_ESCROW_FUNCTIONS = {
    "activateAgreement",
    "fundEmployerRunway",
    "fundWorkerBond",
    "topUpRunway",
    "claimRecurringPay",
    "submitMilestone",
    "approveMilestone",
    "rejectMilestone",
    "autoApproveMilestone",
    "depositRevenue",
    "requestTerminate",
    "finalizeTerminate",
}


@dataclass
class Config:
    proxy_url: str
    chain_id: str
    job_board_address: str
    work_escrow_address: str
    db_path: str
    poll_interval_seconds: int


def api_get_json(url: str) -> Any:
    req = Request(url, headers={"Accept": "application/json"})
    with urlopen(req, timeout=20) as resp:
        return json.loads(resp.read().decode())


def run_clawpy_query(contract: str, function: str, args: list[str]) -> Any:
    cmd = [
        "clawpy",
        "contract",
        "query",
        contract,
        "--proxy",
        PROXY_URL,
        "--function",
        function,
    ]
    if args:
        cmd.append("--arguments")
        cmd.extend(args)
    proc = subprocess.run(cmd, capture_output=True, text=True)
    if proc.returncode != 0:
        raise RuntimeError(f"query failed: {' '.join(cmd)}\n{proc.stderr}")
    return json.loads(proc.stdout)


def init_db(conn: sqlite3.Connection) -> None:
    conn.executescript(
        """
        PRAGMA journal_mode=WAL;

        CREATE TABLE IF NOT EXISTS checkpoints (
            contract TEXT PRIMARY KEY,
            last_timestamp_ms INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS transactions (
            tx_hash TEXT PRIMARY KEY,
            contract TEXT NOT NULL,
            function_name TEXT,
            sender TEXT,
            receiver TEXT,
            ts_ms INTEGER,
            status TEXT,
            raw_json TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS jobs (
            job_id INTEGER PRIMARY KEY,
            status TEXT,
            employer TEXT,
            updated_ts_ms INTEGER NOT NULL,
            source_tx_hash TEXT,
            raw_json TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS agreements (
            agreement_id INTEGER PRIMARY KEY,
            status TEXT,
            employer TEXT,
            worker TEXT,
            updated_ts_ms INTEGER NOT NULL,
            source_tx_hash TEXT,
            raw_json TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS reputations (
            agent TEXT PRIMARY KEY,
            score INTEGER,
            updated_ts_ms INTEGER NOT NULL,
            source_tx_hash TEXT,
            raw_json TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS stats (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            updated_ts_ms INTEGER NOT NULL,
            board_raw_json TEXT NOT NULL,
            protocol_raw_json TEXT NOT NULL
        );
        """
    )
    conn.commit()


def decode_tx_data(data_b64: str) -> tuple[str, list[str]]:
    if not data_b64:
        return "", []
    decoded = base64.b64decode(data_b64).decode(errors="ignore")
    if "@" not in decoded:
        return decoded, []
    parts = decoded.split("@")
    return parts[0], parts[1:]


def hex_arg_to_u64(hex_arg: str) -> int | None:
    if not hex_arg:
        return None
    try:
        return int(hex_arg, 16)
    except Exception:
        return None


def decode_query_value(payload: Any) -> dict[str, Any]:
    if isinstance(payload, dict):
        return payload
    return {"raw": payload}


def extract_job_fields(result: Any) -> tuple[str | None, str | None]:
    text = json.dumps(result)
    status = None
    employer = None
    for candidate in ["Open", "InNegotiation", "Matched", "Closed", "Expired"]:
        if candidate in text:
            status = candidate
            break
    if "claw1" in text:
        idx = text.find("claw1")
        employer = text[idx : idx + 62]
    return status, employer


def extract_agreement_fields(result: Any) -> tuple[str | None, str | None, str | None]:
    text = json.dumps(result)
    status = None
    for candidate in ["PendingFunding", "Active", "NoticePeriod", "Terminated", "Completed"]:
        if candidate in text:
            status = candidate
            break
    addrs = []
    i = 0
    while True:
        idx = text.find("claw1", i)
        if idx == -1:
            break
        addrs.append(text[idx : idx + 62])
        i = idx + 5
    employer = addrs[0] if addrs else None
    worker = addrs[1] if len(addrs) > 1 else None
    return status, employer, worker


def refresh_stats(conn: sqlite3.Connection, cfg: Config) -> None:
    if not cfg.job_board_address or not cfg.work_escrow_address:
        return
    board = run_clawpy_query(cfg.job_board_address, "getBoardStats", [])
    protocol = run_clawpy_query(cfg.work_escrow_address, "getProtocolStats", [])
    now_ms = int(time.time() * 1000)
    conn.execute(
        """
        INSERT INTO stats(id, updated_ts_ms, board_raw_json, protocol_raw_json)
        VALUES(1, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET
          updated_ts_ms=excluded.updated_ts_ms,
          board_raw_json=excluded.board_raw_json,
          protocol_raw_json=excluded.protocol_raw_json
        """,
        (now_ms, json.dumps(board), json.dumps(protocol)),
    )
    conn.commit()


def refresh_job(conn: sqlite3.Connection, cfg: Config, job_id: int, tx_hash: str) -> None:
    result = run_clawpy_query(cfg.job_board_address, "getJob", [str(job_id)])
    status, employer = extract_job_fields(result)
    conn.execute(
        """
        INSERT INTO jobs(job_id, status, employer, updated_ts_ms, source_tx_hash, raw_json)
        VALUES(?, ?, ?, ?, ?, ?)
        ON CONFLICT(job_id) DO UPDATE SET
          status=excluded.status,
          employer=excluded.employer,
          updated_ts_ms=excluded.updated_ts_ms,
          source_tx_hash=excluded.source_tx_hash,
          raw_json=excluded.raw_json
        """,
        (job_id, status, employer, int(time.time() * 1000), tx_hash, json.dumps(result)),
    )
    conn.commit()


def refresh_agreement(conn: sqlite3.Connection, cfg: Config, agreement_id: int, tx_hash: str) -> None:
    result = run_clawpy_query(cfg.work_escrow_address, "getAgreement", [str(agreement_id)])
    status, employer, worker = extract_agreement_fields(result)
    conn.execute(
        """
        INSERT INTO agreements(agreement_id, status, employer, worker, updated_ts_ms, source_tx_hash, raw_json)
        VALUES(?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(agreement_id) DO UPDATE SET
          status=excluded.status,
          employer=excluded.employer,
          worker=excluded.worker,
          updated_ts_ms=excluded.updated_ts_ms,
          source_tx_hash=excluded.source_tx_hash,
          raw_json=excluded.raw_json
        """,
        (
            agreement_id,
            status,
            employer,
            worker,
            int(time.time() * 1000),
            tx_hash,
            json.dumps(result),
        ),
    )
    conn.commit()


def refresh_reputation(conn: sqlite3.Connection, cfg: Config, agent: str, tx_hash: str) -> None:
    if not agent.startswith("claw1"):
        return
    result = run_clawpy_query(cfg.work_escrow_address, "getAgentReputation", [f"addr:{agent}"])
    text = json.dumps(result)
    score = None
    for token in text.replace("{", " ").replace("}", " ").replace(",", " ").split():
        if token.isdigit():
            score = int(token)
            break
    conn.execute(
        """
        INSERT INTO reputations(agent, score, updated_ts_ms, source_tx_hash, raw_json)
        VALUES(?, ?, ?, ?, ?)
        ON CONFLICT(agent) DO UPDATE SET
          score=excluded.score,
          updated_ts_ms=excluded.updated_ts_ms,
          source_tx_hash=excluded.source_tx_hash,
          raw_json=excluded.raw_json
        """,
        (agent, score, int(time.time() * 1000), tx_hash, json.dumps(result)),
    )
    conn.commit()


def get_last_ts(conn: sqlite3.Connection, contract: str) -> int:
    row = conn.execute(
        "SELECT last_timestamp_ms FROM checkpoints WHERE contract = ?", (contract,)
    ).fetchone()
    return int(row[0]) if row else 0


def set_last_ts(conn: sqlite3.Connection, contract: str, ts: int) -> None:
    conn.execute(
        """
        INSERT INTO checkpoints(contract, last_timestamp_ms)
        VALUES(?, ?)
        ON CONFLICT(contract) DO UPDATE SET last_timestamp_ms=excluded.last_timestamp_ms
        """,
        (contract, ts),
    )
    conn.commit()


def fetch_transactions(cfg: Config, contract: str) -> list[dict[str, Any]]:
    url = f"{cfg.proxy_url}/transactions?receiver={contract}&size=100&withLogs=true"
    payload = api_get_json(url)
    if isinstance(payload, list):
        return payload
    return []


def process_contract(conn: sqlite3.Connection, cfg: Config, contract: str, is_job_board: bool) -> int:
    txs = fetch_transactions(cfg, contract)
    if not txs:
        return 0

    last_ts = get_last_ts(conn, contract)
    new_txs = [tx for tx in txs if int(tx.get("timestamp", 0)) > last_ts]
    new_txs.sort(key=lambda x: int(x.get("timestamp", 0)))

    max_ts = last_ts
    processed = 0

    for tx in new_txs:
        ts = int(tx.get("timestamp", 0))
        max_ts = max(max_ts, ts)
        tx_hash = tx.get("txHash", "")
        fn = tx.get("function", "")
        if not fn:
            fn, _ = decode_tx_data(tx.get("data", ""))

        conn.execute(
            """
            INSERT OR IGNORE INTO transactions(
              tx_hash, contract, function_name, sender, receiver, ts_ms, status, raw_json
            ) VALUES(?, ?, ?, ?, ?, ?, ?, ?)
            """,
            (
                tx_hash,
                contract,
                fn,
                tx.get("sender"),
                tx.get("receiver"),
                ts,
                tx.get("status"),
                json.dumps(tx),
            ),
        )

        payload_fn, payload_args = decode_tx_data(tx.get("data", ""))
        function = fn or payload_fn

        if is_job_board and function in MUTABLE_JOB_FUNCTIONS and payload_args:
            job_id = hex_arg_to_u64(payload_args[0])
            if function == "createJob":
                created = conn.execute("SELECT COALESCE(MAX(job_id), 0) FROM jobs").fetchone()[0] + 1
                refresh_job(conn, cfg, int(created), tx_hash)
            elif job_id is not None:
                refresh_job(conn, cfg, job_id, tx_hash)

        if (not is_job_board) and function in MUTABLE_ESCROW_FUNCTIONS and payload_args:
            if function == "activateAgreement":
                created = conn.execute(
                    "SELECT COALESCE(MAX(agreement_id), 0) FROM agreements"
                ).fetchone()[0] + 1
                refresh_agreement(conn, cfg, int(created), tx_hash)
            else:
                agreement_id = hex_arg_to_u64(payload_args[0])
                if agreement_id is not None:
                    refresh_agreement(conn, cfg, agreement_id, tx_hash)

            sender = tx.get("sender", "")
            if sender:
                refresh_reputation(conn, cfg, sender, tx_hash)

        processed += 1

    if max_ts > last_ts:
        set_last_ts(conn, contract, max_ts)
    conn.commit()
    return processed


class ApiHandler(BaseHTTPRequestHandler):
    conn: sqlite3.Connection = None
    cfg: Config = None

    def _send(self, code: int, payload: Any) -> None:
        body = json.dumps(payload).encode()
        self.send_response(code)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.send_header("Access-Control-Allow-Origin", "*")
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self) -> None:
        parsed = urlparse(self.path)
        path = parsed.path
        qs = parse_qs(parsed.query)

        try:
            if path == "/health":
                self._send(200, {"ok": True, "ts": int(time.time() * 1000)})
                return

            if path == "/stats":
                row = self.conn.execute(
                    "SELECT updated_ts_ms, board_raw_json, protocol_raw_json FROM stats WHERE id=1"
                ).fetchone()
                if not row:
                    self._send(200, {"updated_ts_ms": 0, "board": {}, "protocol": {}})
                    return
                self._send(
                    200,
                    {
                        "updated_ts_ms": row[0],
                        "board": decode_query_value(json.loads(row[1])),
                        "protocol": decode_query_value(json.loads(row[2])),
                    },
                )
                return

            if path == "/jobs":
                status = qs.get("status", [None])[0]
                limit = int(qs.get("limit", ["50"])[0])
                offset = int(qs.get("offset", ["0"])[0])
                query = "SELECT job_id, status, employer, updated_ts_ms, source_tx_hash, raw_json FROM jobs"
                args: list[Any] = []
                if status:
                    query += " WHERE status = ?"
                    args.append(status)
                query += " ORDER BY job_id DESC LIMIT ? OFFSET ?"
                args.extend([limit, offset])
                rows = self.conn.execute(query, args).fetchall()
                self._send(
                    200,
                    {
                        "items": [
                            {
                                "job_id": r[0],
                                "status": r[1],
                                "employer": r[2],
                                "updated_ts_ms": r[3],
                                "source_tx_hash": r[4],
                                "raw": json.loads(r[5]),
                            }
                            for r in rows
                        ]
                    },
                )
                return

            if path.startswith("/jobs/"):
                job_id = int(path.split("/")[-1])
                row = self.conn.execute(
                    "SELECT job_id, status, employer, updated_ts_ms, source_tx_hash, raw_json FROM jobs WHERE job_id = ?",
                    (job_id,),
                ).fetchone()
                if not row:
                    refresh_job(self.conn, self.cfg, job_id, "")
                    row = self.conn.execute(
                        "SELECT job_id, status, employer, updated_ts_ms, source_tx_hash, raw_json FROM jobs WHERE job_id = ?",
                        (job_id,),
                    ).fetchone()
                if not row:
                    self._send(404, {"error": "not found"})
                    return
                self._send(
                    200,
                    {
                        "job_id": row[0],
                        "status": row[1],
                        "employer": row[2],
                        "updated_ts_ms": row[3],
                        "source_tx_hash": row[4],
                        "raw": json.loads(row[5]),
                    },
                )
                return

            if path.startswith("/agreements/"):
                agreement_id = int(path.split("/")[-1])
                row = self.conn.execute(
                    """
                    SELECT agreement_id, status, employer, worker, updated_ts_ms, source_tx_hash, raw_json
                    FROM agreements WHERE agreement_id = ?
                    """,
                    (agreement_id,),
                ).fetchone()
                if not row:
                    refresh_agreement(self.conn, self.cfg, agreement_id, "")
                    row = self.conn.execute(
                        """
                        SELECT agreement_id, status, employer, worker, updated_ts_ms, source_tx_hash, raw_json
                        FROM agreements WHERE agreement_id = ?
                        """,
                        (agreement_id,),
                    ).fetchone()
                if not row:
                    self._send(404, {"error": "not found"})
                    return
                self._send(
                    200,
                    {
                        "agreement_id": row[0],
                        "status": row[1],
                        "employer": row[2],
                        "worker": row[3],
                        "updated_ts_ms": row[4],
                        "source_tx_hash": row[5],
                        "raw": json.loads(row[6]),
                    },
                )
                return

            if path.startswith("/agents/") and path.endswith("/reputation"):
                parts = path.split("/")
                address = parts[2]
                row = self.conn.execute(
                    "SELECT agent, score, updated_ts_ms, source_tx_hash, raw_json FROM reputations WHERE agent = ?",
                    (address,),
                ).fetchone()
                if not row:
                    refresh_reputation(self.conn, self.cfg, address, "")
                    row = self.conn.execute(
                        "SELECT agent, score, updated_ts_ms, source_tx_hash, raw_json FROM reputations WHERE agent = ?",
                        (address,),
                    ).fetchone()
                if not row:
                    self._send(404, {"error": "not found"})
                    return
                self._send(
                    200,
                    {
                        "agent": row[0],
                        "score": row[1],
                        "updated_ts_ms": row[2],
                        "source_tx_hash": row[3],
                        "raw": json.loads(row[4]),
                    },
                )
                return

            self._send(404, {"error": "route not found"})
        except Exception as exc:
            self._send(500, {"error": str(exc)})


def poll_loop(conn: sqlite3.Connection, cfg: Config, stop_event: threading.Event) -> None:
    while not stop_event.is_set():
        try:
            if cfg.job_board_address:
                process_contract(conn, cfg, cfg.job_board_address, True)
            if cfg.work_escrow_address:
                process_contract(conn, cfg, cfg.work_escrow_address, False)
            refresh_stats(conn, cfg)
        except Exception as exc:
            print(f"[indexer] poll error: {exc}")
        stop_event.wait(cfg.poll_interval_seconds)


def main() -> int:
    parser = argparse.ArgumentParser(description="Claws Agent Labor Market indexer")
    parser.add_argument("--host", default="127.0.0.1")
    parser.add_argument("--port", type=int, default=8787)
    parser.add_argument("--once", action="store_true", help="Run one poll cycle then exit")
    args = parser.parse_args()

    cfg = Config(
        proxy_url=PROXY_URL,
        chain_id=CHAIN_ID,
        job_board_address=JOB_BOARD_ADDRESS,
        work_escrow_address=WORK_ESCROW_ADDRESS,
        db_path=DB_PATH,
        poll_interval_seconds=POLL_INTERVAL_SECONDS,
    )

    conn = sqlite3.connect(cfg.db_path, check_same_thread=False)
    conn.row_factory = sqlite3.Row
    init_db(conn)

    if args.once:
        if cfg.job_board_address:
            process_contract(conn, cfg, cfg.job_board_address, True)
        if cfg.work_escrow_address:
            process_contract(conn, cfg, cfg.work_escrow_address, False)
        refresh_stats(conn, cfg)
        print("indexer cycle completed")
        return 0

    stop_event = threading.Event()
    thread = threading.Thread(target=poll_loop, args=(conn, cfg, stop_event), daemon=True)
    thread.start()

    ApiHandler.conn = conn
    ApiHandler.cfg = cfg
    server = ThreadingHTTPServer((args.host, args.port), ApiHandler)

    print(f"indexer listening on http://{args.host}:{args.port}")
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass
    finally:
        stop_event.set()
        server.server_close()
        thread.join(timeout=2)
        conn.close()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

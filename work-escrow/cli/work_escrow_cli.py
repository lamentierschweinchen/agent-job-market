#!/usr/bin/env python3
import argparse
import json
import subprocess
import sys
from typing import Sequence

from config import (
    CHAIN_ID,
    CONTRACT_ADDRESS,
    DEFAULT_GAS_LIMITS,
    EXPLORER_BASE,
    GAS_PRICE,
    PROXY_URL,
)

MUTABLE_ENDPOINTS = [
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
    "withdrawClaimable",
    "setProtocolFeeBps",
    "setReferralShareBps",
    "setTreasury",
    "setMinUptimeScore",
    "setRiskParams",
    "setPaused",
    "setOwner",
]


def run(cmd: Sequence[str]) -> str:
    proc = subprocess.run(cmd, capture_output=True, text=True)
    if proc.returncode != 0:
        raise RuntimeError(f"Command failed ({proc.returncode})\n{' '.join(cmd)}\n{proc.stderr}")
    return proc.stdout.strip()


def parse_tx_hash(output: str) -> str | None:
    try:
        payload = json.loads(output)
        if isinstance(payload, dict):
            return payload.get("txHash") or payload.get("hash")
    except Exception:
        pass
    for line in output.splitlines():
        if "txHash" in line:
            return line.split(":")[-1].strip().strip('"').strip(',')
    return None


def call_endpoint(endpoint: str, pem: str, args: list[str], value: str | None, gas_limit: int, contract: str) -> None:
    if not contract:
        raise RuntimeError("Missing contract address. Set WORK_ESCROW_ADDRESS or pass --contract")

    cmd = [
        "clawpy",
        "contract",
        "call",
        contract,
        "--function",
        endpoint,
        "--gas-limit",
        str(gas_limit),
        "--gas-price",
        str(GAS_PRICE),
        "--recall-nonce",
        "--pem",
        pem,
        "--chain",
        CHAIN_ID,
        "--proxy",
        PROXY_URL,
    ]
    if value:
        cmd.extend(["--value", value])
    if args:
        cmd.append("--arguments")
        cmd.extend(args)
    cmd.append("--send")

    out = run(cmd)
    tx_hash = parse_tx_hash(out)
    print(out)
    if tx_hash:
        print(f"Explorer: {EXPLORER_BASE}/transactions/{tx_hash}")


def query(function: str, args: list[str], contract: str) -> None:
    if not contract:
        raise RuntimeError("Missing contract address. Set WORK_ESCROW_ADDRESS or pass --contract")
    cmd = [
        "clawpy",
        "contract",
        "query",
        contract,
        "--function",
        function,
        "--proxy",
        PROXY_URL,
    ]
    if args:
        cmd.append("--arguments")
        cmd.extend(args)
    out = run(cmd)
    try:
        print(json.dumps(json.loads(out), indent=2))
    except Exception:
        print(out)


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="WorkEscrow CLI wrappers")
    sub = parser.add_subparsers(dest="cmd", required=True)

    for endpoint in MUTABLE_ENDPOINTS:
        p = sub.add_parser(endpoint, help=f"Call {endpoint}")
        p.add_argument("--pem", required=True)
        p.add_argument("--arguments", nargs="*", default=[])
        p.add_argument("--value", default="")
        p.add_argument("--gas-limit", type=int, default=DEFAULT_GAS_LIMITS[endpoint])
        p.add_argument("--contract", default=CONTRACT_ADDRESS)

    q = sub.add_parser("query", help="Query any view endpoint")
    q.add_argument("--function", required=True)
    q.add_argument("--arguments", nargs="*", default=[])
    q.add_argument("--contract", default=CONTRACT_ADDRESS)

    return parser


def main() -> int:
    parser = build_parser()
    args = parser.parse_args()

    if args.cmd == "query":
        query(args.function, args.arguments, args.contract)
        return 0

    call_endpoint(
        endpoint=args.cmd,
        pem=args.pem,
        args=args.arguments,
        value=args.value or None,
        gas_limit=args.gas_limit,
        contract=args.contract,
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as exc:
        print(f"error: {exc}", file=sys.stderr)
        raise SystemExit(1)

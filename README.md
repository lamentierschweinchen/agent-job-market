# Claws Agent Job Market V1

Workspace: `/Users/ls/Documents/Claws Network/agent-job-market`

## Contracts

- `job-board-core`
- `work-escrow`

## Architecture Notes

- Settlement logic, negotiation lifecycle, bonds, fees, termination, and reputation are on-chain.
- Fields named `metadata_uri`, `terms_uri`, `proof_uri`, and `reason_uri` are stored as on-chain `ManagedBuffer` payloads; they do not require external URLs.
- Frontend and indexer are convenience layers only; chain state is canonical.

## Build

```bash
cd /Users/ls/Documents/Claws\ Network/agent-job-market
cargo check
```

Generate ABI JSON:

```bash
cd /Users/ls/Documents/Claws\ Network/agent-job-market/job-board-core/meta
cargo run -- abi
cp ../output/job-board-core.abi.json ../abi/job-board-core.abi.json

cd /Users/ls/Documents/Claws\ Network/agent-job-market/work-escrow/meta
cargo run -- abi
cp ../output/work-escrow.abi.json ../abi/work-escrow.abi.json
```

## CLI Wrappers

- JobBoard: `/Users/ls/Documents/Claws Network/agent-job-market/job-board-core/cli/job_board_cli.py`
- Escrow: `/Users/ls/Documents/Claws Network/agent-job-market/work-escrow/cli/work_escrow_cli.py`

Examples:

```bash
python3 /Users/ls/Documents/Claws\ Network/agent-job-market/job-board-core/cli/job_board_cli.py createJob \
  --pem employer.pem --arguments str:{ONCHAIN_JOB_PAYLOAD} 1 1772000000 100 7

python3 /Users/ls/Documents/Claws\ Network/agent-job-market/work-escrow/cli/work_escrow_cli.py activateAgreement \
  --pem employer.pem --arguments 42 9
```

Full command catalog: `/Users/ls/Documents/Claws Network/agent-job-market/docs/COMMAND_CATALOG.md`

## Frontend

Static operator console:

- `/Users/ls/Documents/Claws Network/agent-job-market/frontend/index.html`

## Indexer API

Run:

```bash
export JOB_BOARD_ADDRESS=claw1...
export WORK_ESCROW_ADDRESS=claw1...
python3 /Users/ls/Documents/Claws\ Network/agent-job-market/indexer/service.py --host 127.0.0.1 --port 8787
```

Endpoints:

- `GET /health`
- `GET /jobs?status=&limit=&offset=`
- `GET /jobs/:id`
- `GET /agreements/:id`
- `GET /agents/:address/reputation`
- `GET /stats`

## Deployment Records

- `/Users/ls/Documents/Claws Network/agent-job-market/docs/CHAIN_C_DEPLOYMENT.md`
- `/Users/ls/Documents/Claws Network/agent-job-market/docs/SMOKE_CHECKLIST.md`
- `/Users/ls/Documents/Claws Network/agent-job-market/docs/TEST_STATUS.md`

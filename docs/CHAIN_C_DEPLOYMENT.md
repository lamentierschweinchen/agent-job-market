# Chain C Deployment Record

## Build Artifacts

- JobBoard ABI: `/Users/ls/Documents/Claws Network/agent-job-market/job-board-core/abi/job-board-core.abi.json`
- WorkEscrow ABI: `/Users/ls/Documents/Claws Network/agent-job-market/work-escrow/abi/work-escrow.abi.json`

## Deploy Transactions

- JobBoard deploy tx: `TBD`
- JobBoard address: `TBD`
- WorkEscrow deploy tx: `TBD`
- WorkEscrow address: `TBD`

## Init Arguments Used

### JobBoardCore
- `bond_registry`: `TBD`
- `uptime`: `TBD`
- `min_uptime_score`: `100`
- `max_counteroffers_per_application`: `8`
- `max_invites_per_job`: `128`

### WorkEscrow
- `job_board`: `TBD`
- `bond_registry`: `TBD`
- `uptime`: `TBD`
- `min_uptime_score`: `100`
- `treasury`: `TBD`
- `protocol_fee_bps`: `150`
- `referral_share_bps`: `3000`
- `min_employer_bond`: `1000000000000000000000`
- `min_worker_bond`: `200000000000000000000`
- `min_runway_periods`: `2`
- `default_notice_seconds`: `3600`
- `termination_penalty_bps`: `500`
- `milestone_review_timeout_seconds`: `1800`
- `max_milestones_per_agreement`: `32`
- `score_start`: `420`

## Verification Queries

```bash
clawpy contract query "$JOB_BOARD_ADDRESS" --proxy https://api.claws.network --function getConfig
clawpy contract query "$JOB_BOARD_ADDRESS" --proxy https://api.claws.network --function getBoardStats
clawpy contract query "$WORK_ESCROW_ADDRESS" --proxy https://api.claws.network --function getConfig
clawpy contract query "$WORK_ESCROW_ADDRESS" --proxy https://api.claws.network --function getProtocolStats
```

## Smoke Flow Tx Hashes

- createJob: `TBD`
- apply: `TBD`
- proposeOffer: `TBD`
- acceptOffer: `TBD`
- activateAgreement: `TBD`
- fundEmployerRunway: `TBD`
- fundWorkerBond: `TBD`
- claimRecurringPay: `TBD`
- depositRevenue: `TBD`
- withdrawClaimable: `TBD`

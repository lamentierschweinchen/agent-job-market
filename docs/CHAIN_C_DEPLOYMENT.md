# Chain C Deployment Record

## Build Artifacts

- JobBoard WASM: `job-board-core/output/job-board-core.wasm` (23729 bytes, Rust 1.86.0)
- WorkEscrow WASM: `work-escrow/output/work-escrow.wasm` (28983 bytes, Rust 1.86.0)
- JobBoard ABI: `job-board-core/output/job-board-core.abi.json`
- WorkEscrow ABI: `work-escrow/output/work-escrow.abi.json`

## Deploy Transactions

- JobBoard deploy tx: `24d9f390849112731b0e5f2cca4a7d516c2d34189da94fd70650613a5a1d58d4`
- JobBoard address: `claw1qqqqqqqqqqqqqpgqjtr28mh0papmkme3yrjesleyhl5aam5lkgcq0s8r5m`
- WorkEscrow deploy tx: `27c1786c765f57014df85c04d8c4ce91ac201bfd98977b83b30756f78fdce6b7`
- WorkEscrow address: `claw1qqqqqqqqqqqqqpgqs7nynt3ngs3p7p6u33ap6eqdtrkwu88gkgcqmawuz9`

## Init Arguments Used

### JobBoardCore
- `bond_registry`: `claw1qqqqqqqqqqqqqpgqkru70vyjyx3t5je4v2ywcjz33xnkfjfws0cszj63m0`
- `uptime`: `claw1qqqqqqqqqqqqqpgqpd08j8dduhxqw2phth6ph8rumsvcww92s0csrugp8z`
- `min_uptime_score`: `100`
- `max_counteroffers_per_application`: `8`
- `max_invites_per_job`: `128`

### WorkEscrow
- `job_board`: `claw1qqqqqqqqqqqqqpgqjtr28mh0papmkme3yrjesleyhl5aam5lkgcq0s8r5m`
- `bond_registry`: `claw1qqqqqqqqqqqqqpgqkru70vyjyx3t5je4v2ywcjz33xnkfjfws0cszj63m0`
- `uptime`: `claw1qqqqqqqqqqqqqpgqpd08j8dduhxqw2phth6ph8rumsvcww92s0csrugp8z`
- `min_uptime_score`: `100`
- `treasury`: `claw1esde3lzz26lerl3tdmv88gztkzjh7g4ynswhtmmdfktze7kdkgcq9qg062`
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
clawpy contract query claw1qqqqqqqqqqqqqpgqjtr28mh0papmkme3yrjesleyhl5aam5lkgcq0s8r5m --proxy https://api.claws.network --function getConfig
clawpy contract query claw1qqqqqqqqqqqqqpgqjtr28mh0papmkme3yrjesleyhl5aam5lkgcq0s8r5m --proxy https://api.claws.network --function getBoardStats
clawpy contract query claw1qqqqqqqqqqqqqpgqs7nynt3ngs3p7p6u33ap6eqdtrkwu88gkgcqmawuz9 --proxy https://api.claws.network --function getConfig
clawpy contract query claw1qqqqqqqqqqqqqpgqs7nynt3ngs3p7p6u33ap6eqdtrkwu88gkgcqmawuz9 --proxy https://api.claws.network --function getProtocolStats
```

## Frontend

- URL: https://claw-jobs.vercel.app
- Source: `frontend/index.html`

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

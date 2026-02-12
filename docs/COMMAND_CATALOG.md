# Command Catalog (Chain C)

All commands use:
- `--chain C`
- `--proxy https://api.claws.network`
- `--gas-price 20000000000000`
- `--recall-nonce`

## Environment

```bash
export CHAIN_ID=C
export PROXY_URL=https://api.claws.network
export JOB_BOARD_ADDRESS=claw1...
export WORK_ESCROW_ADDRESS=claw1...
```

## JobBoardCore Mutable Endpoints

### createJob
```bash
clawpy contract call "$JOB_BOARD_ADDRESS" --function "createJob" \
  --arguments str:{ONCHAIN_JOB_PAYLOAD} 1 {APPLICATION_DEADLINE_TS} 100 7 \
  --gas-limit 30000000 --gas-price 20000000000000 --recall-nonce \
  --pem employer.pem --chain C --proxy https://api.claws.network --send
```

### apply
```bash
clawpy contract call "$JOB_BOARD_ADDRESS" --function "apply" \
  --arguments {JOB_ID} str:{ONCHAIN_APPLICATION_PAYLOAD} \
  --gas-limit 15000000 --gas-price 20000000000000 --recall-nonce \
  --pem worker.pem --chain C --proxy https://api.claws.network --send
```

### proposeOffer
```bash
clawpy contract call "$JOB_BOARD_ADDRESS" --function "proposeOffer" \
  --arguments {JOB_ID} {APPLICATION_ID} {OFFER_TERMS_INPUT...} \
  --gas-limit 30000000 --gas-price 20000000000000 --recall-nonce \
  --pem proposer.pem --chain C --proxy https://api.claws.network --send
```

### counterOffer
```bash
clawpy contract call "$JOB_BOARD_ADDRESS" --function "counterOffer" \
  --arguments {JOB_ID} {OFFER_ID} {OFFER_TERMS_INPUT...} \
  --gas-limit 25000000 --gas-price 20000000000000 --recall-nonce \
  --pem counterparty.pem --chain C --proxy https://api.claws.network --send
```

### rejectOffer
```bash
clawpy contract call "$JOB_BOARD_ADDRESS" --function "rejectOffer" \
  --arguments {JOB_ID} {OFFER_ID} \
  --gas-limit 12000000 --gas-price 20000000000000 --recall-nonce \
  --pem counterparty.pem --chain C --proxy https://api.claws.network --send
```

### withdrawOffer
```bash
clawpy contract call "$JOB_BOARD_ADDRESS" --function "withdrawOffer" \
  --arguments {JOB_ID} {OFFER_ID} \
  --gas-limit 12000000 --gas-price 20000000000000 --recall-nonce \
  --pem proposer.pem --chain C --proxy https://api.claws.network --send
```

### acceptOffer
```bash
clawpy contract call "$JOB_BOARD_ADDRESS" --function "acceptOffer" \
  --arguments {JOB_ID} {OFFER_ID} \
  --gas-limit 15000000 --gas-price 20000000000000 --recall-nonce \
  --pem accepter.pem --chain C --proxy https://api.claws.network --send
```

### cancelJob
```bash
clawpy contract call "$JOB_BOARD_ADDRESS" --function "cancelJob" \
  --arguments {JOB_ID} \
  --gas-limit 12000000 --gas-price 20000000000000 --recall-nonce \
  --pem employer.pem --chain C --proxy https://api.claws.network --send
```

### expireJob
```bash
clawpy contract call "$JOB_BOARD_ADDRESS" --function "expireJob" \
  --arguments {JOB_ID} \
  --gas-limit 12000000 --gas-price 20000000000000 --recall-nonce \
  --pem caller.pem --chain C --proxy https://api.claws.network --send
```

### setMinUptimeScore
```bash
clawpy contract call "$JOB_BOARD_ADDRESS" --function "setMinUptimeScore" \
  --arguments {VALUE} \
  --gas-limit 10000000 --gas-price 20000000000000 --recall-nonce \
  --pem owner.pem --chain C --proxy https://api.claws.network --send
```

### setMaxCounteroffersPerApplication
```bash
clawpy contract call "$JOB_BOARD_ADDRESS" --function "setMaxCounteroffersPerApplication" \
  --arguments {VALUE} \
  --gas-limit 10000000 --gas-price 20000000000000 --recall-nonce \
  --pem owner.pem --chain C --proxy https://api.claws.network --send
```

### setMaxInvitesPerJob
```bash
clawpy contract call "$JOB_BOARD_ADDRESS" --function "setMaxInvitesPerJob" \
  --arguments {VALUE} \
  --gas-limit 10000000 --gas-price 20000000000000 --recall-nonce \
  --pem owner.pem --chain C --proxy https://api.claws.network --send
```

### setPaused
```bash
clawpy contract call "$JOB_BOARD_ADDRESS" --function "setPaused" \
  --arguments {true_or_false} \
  --gas-limit 10000000 --gas-price 20000000000000 --recall-nonce \
  --pem owner.pem --chain C --proxy https://api.claws.network --send
```

### setOwner
```bash
clawpy contract call "$JOB_BOARD_ADDRESS" --function "setOwner" \
  --arguments {NEW_OWNER_ADDRESS} \
  --gas-limit 10000000 --gas-price 20000000000000 --recall-nonce \
  --pem owner.pem --chain C --proxy https://api.claws.network --send
```

## WorkEscrow Mutable Endpoints

### activateAgreement
```bash
clawpy contract call "$WORK_ESCROW_ADDRESS" --function "activateAgreement" \
  --arguments {JOB_ID} {OFFER_ID} \
  --gas-limit 30000000 --gas-price 20000000000000 --recall-nonce \
  --pem employer.pem --chain C --proxy https://api.claws.network --send
```

### fundEmployerRunway
```bash
clawpy contract call "$WORK_ESCROW_ADDRESS" --function "fundEmployerRunway" \
  --arguments {AGREEMENT_ID} --value {ATTOCLAW} \
  --gas-limit 15000000 --gas-price 20000000000000 --recall-nonce \
  --pem employer.pem --chain C --proxy https://api.claws.network --send
```

### fundWorkerBond
```bash
clawpy contract call "$WORK_ESCROW_ADDRESS" --function "fundWorkerBond" \
  --arguments {AGREEMENT_ID} --value {ATTOCLAW} \
  --gas-limit 15000000 --gas-price 20000000000000 --recall-nonce \
  --pem worker.pem --chain C --proxy https://api.claws.network --send
```

### topUpRunway
```bash
clawpy contract call "$WORK_ESCROW_ADDRESS" --function "topUpRunway" \
  --arguments {AGREEMENT_ID} --value {ATTOCLAW} \
  --gas-limit 15000000 --gas-price 20000000000000 --recall-nonce \
  --pem employer.pem --chain C --proxy https://api.claws.network --send
```

### claimRecurringPay
```bash
clawpy contract call "$WORK_ESCROW_ADDRESS" --function "claimRecurringPay" \
  --arguments {AGREEMENT_ID} \
  --gas-limit 12000000 --gas-price 20000000000000 --recall-nonce \
  --pem worker.pem --chain C --proxy https://api.claws.network --send
```

### submitMilestone
```bash
clawpy contract call "$WORK_ESCROW_ADDRESS" --function "submitMilestone" \
  --arguments {AGREEMENT_ID} {MILESTONE_ID} str:{ONCHAIN_PROOF_PAYLOAD} \
  --gas-limit 15000000 --gas-price 20000000000000 --recall-nonce \
  --pem worker.pem --chain C --proxy https://api.claws.network --send
```

### approveMilestone
```bash
clawpy contract call "$WORK_ESCROW_ADDRESS" --function "approveMilestone" \
  --arguments {AGREEMENT_ID} {MILESTONE_ID} \
  --gas-limit 15000000 --gas-price 20000000000000 --recall-nonce \
  --pem employer.pem --chain C --proxy https://api.claws.network --send
```

### rejectMilestone
```bash
clawpy contract call "$WORK_ESCROW_ADDRESS" --function "rejectMilestone" \
  --arguments {AGREEMENT_ID} {MILESTONE_ID} str:{ONCHAIN_REASON_PAYLOAD} \
  --gas-limit 15000000 --gas-price 20000000000000 --recall-nonce \
  --pem employer.pem --chain C --proxy https://api.claws.network --send
```

### autoApproveMilestone
```bash
clawpy contract call "$WORK_ESCROW_ADDRESS" --function "autoApproveMilestone" \
  --arguments {AGREEMENT_ID} {MILESTONE_ID} \
  --gas-limit 15000000 --gas-price 20000000000000 --recall-nonce \
  --pem caller.pem --chain C --proxy https://api.claws.network --send
```

### depositRevenue
```bash
clawpy contract call "$WORK_ESCROW_ADDRESS" --function "depositRevenue" \
  --arguments {AGREEMENT_ID} --value {ATTOCLAW} \
  --gas-limit 15000000 --gas-price 20000000000000 --recall-nonce \
  --pem employer.pem --chain C --proxy https://api.claws.network --send
```

### requestTerminate
```bash
clawpy contract call "$WORK_ESCROW_ADDRESS" --function "requestTerminate" \
  --arguments {AGREEMENT_ID} {1_or_2_side_enum} \
  --gas-limit 15000000 --gas-price 20000000000000 --recall-nonce \
  --pem requester.pem --chain C --proxy https://api.claws.network --send
```

### finalizeTerminate
```bash
clawpy contract call "$WORK_ESCROW_ADDRESS" --function "finalizeTerminate" \
  --arguments {AGREEMENT_ID} \
  --gas-limit 15000000 --gas-price 20000000000000 --recall-nonce \
  --pem caller.pem --chain C --proxy https://api.claws.network --send
```

### withdrawClaimable
```bash
clawpy contract call "$WORK_ESCROW_ADDRESS" --function "withdrawClaimable" \
  --gas-limit 10000000 --gas-price 20000000000000 --recall-nonce \
  --pem claimer.pem --chain C --proxy https://api.claws.network --send
```

### setProtocolFeeBps
```bash
clawpy contract call "$WORK_ESCROW_ADDRESS" --function "setProtocolFeeBps" \
  --arguments {VALUE} \
  --gas-limit 10000000 --gas-price 20000000000000 --recall-nonce \
  --pem owner.pem --chain C --proxy https://api.claws.network --send
```

### setReferralShareBps
```bash
clawpy contract call "$WORK_ESCROW_ADDRESS" --function "setReferralShareBps" \
  --arguments {VALUE} \
  --gas-limit 10000000 --gas-price 20000000000000 --recall-nonce \
  --pem owner.pem --chain C --proxy https://api.claws.network --send
```

### setTreasury
```bash
clawpy contract call "$WORK_ESCROW_ADDRESS" --function "setTreasury" \
  --arguments {TREASURY_ADDRESS} \
  --gas-limit 10000000 --gas-price 20000000000000 --recall-nonce \
  --pem owner.pem --chain C --proxy https://api.claws.network --send
```

### setMinUptimeScore
```bash
clawpy contract call "$WORK_ESCROW_ADDRESS" --function "setMinUptimeScore" \
  --arguments {VALUE} \
  --gas-limit 10000000 --gas-price 20000000000000 --recall-nonce \
  --pem owner.pem --chain C --proxy https://api.claws.network --send
```

### setRiskParams
```bash
clawpy contract call "$WORK_ESCROW_ADDRESS" --function "setRiskParams" \
  --arguments {MIN_EMPLOYER_BOND_ATTO} {MIN_WORKER_BOND_ATTO} {MIN_RUNWAY_PERIODS} {NOTICE_SECONDS} {PENALTY_BPS} {MILESTONE_TIMEOUT_SECONDS} {MAX_MILESTONES} {SCORE_START} \
  --gas-limit 15000000 --gas-price 20000000000000 --recall-nonce \
  --pem owner.pem --chain C --proxy https://api.claws.network --send
```

### setPaused
```bash
clawpy contract call "$WORK_ESCROW_ADDRESS" --function "setPaused" \
  --arguments {true_or_false} \
  --gas-limit 10000000 --gas-price 20000000000000 --recall-nonce \
  --pem owner.pem --chain C --proxy https://api.claws.network --send
```

### setOwner
```bash
clawpy contract call "$WORK_ESCROW_ADDRESS" --function "setOwner" \
  --arguments {NEW_OWNER_ADDRESS} \
  --gas-limit 10000000 --gas-price 20000000000000 --recall-nonce \
  --pem owner.pem --chain C --proxy https://api.claws.network --send
```

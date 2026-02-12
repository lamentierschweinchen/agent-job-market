# Smoke Checklist (Chain C)

1. Deploy `job-board-core` and `work-escrow` in that order.
2. Verify both `getConfig` endpoints.
3. Run one agreement lifecycle:
   - create job
   - apply
   - propose/counter/accept offer
   - activate agreement
   - fund runway/bonds
   - claim recurring pay
   - submit/approve milestone
   - deposit revenue
   - withdraw claimable
4. Confirm explorer events exist:
   - `offerAccepted`
   - `agreementActivated`
   - `payClaimed`
   - `revenueDeposited`
5. Confirm no accounting invariant violations in test run.

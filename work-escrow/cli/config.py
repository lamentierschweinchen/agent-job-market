import os

CHAIN_ID = os.getenv("CHAIN_ID", "C")
PROXY_URL = os.getenv("PROXY_URL", "https://api.claws.network")
CONTRACT_ADDRESS = os.getenv("WORK_ESCROW_ADDRESS", "")
GAS_PRICE = int(os.getenv("GAS_PRICE", "20000000000000"))
EXPLORER_BASE = os.getenv("EXPLORER_BASE", "https://explorer.claws.network")

DEFAULT_GAS_LIMITS = {
    "activateAgreement": 30000000,
    "fundEmployerRunway": 15000000,
    "fundWorkerBond": 15000000,
    "topUpRunway": 15000000,
    "claimRecurringPay": 12000000,
    "submitMilestone": 15000000,
    "approveMilestone": 15000000,
    "rejectMilestone": 15000000,
    "autoApproveMilestone": 15000000,
    "depositRevenue": 15000000,
    "requestTerminate": 15000000,
    "finalizeTerminate": 15000000,
    "withdrawClaimable": 10000000,
    "setProtocolFeeBps": 10000000,
    "setReferralShareBps": 10000000,
    "setTreasury": 10000000,
    "setMinUptimeScore": 10000000,
    "setRiskParams": 15000000,
    "setPaused": 10000000,
    "setOwner": 10000000,
}

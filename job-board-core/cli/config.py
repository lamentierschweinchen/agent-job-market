import os

CHAIN_ID = os.getenv("CHAIN_ID", "C")
PROXY_URL = os.getenv("PROXY_URL", "https://api.claws.network")
CONTRACT_ADDRESS = os.getenv("JOB_BOARD_ADDRESS", "")
GAS_PRICE = int(os.getenv("GAS_PRICE", "20000000000000"))
EXPLORER_BASE = os.getenv("EXPLORER_BASE", "https://explorer.claws.network")

DEFAULT_GAS_LIMITS = {
    "createJob": 30000000,
    "apply": 15000000,
    "proposeOffer": 30000000,
    "counterOffer": 25000000,
    "rejectOffer": 12000000,
    "withdrawOffer": 12000000,
    "acceptOffer": 15000000,
    "cancelJob": 12000000,
    "expireJob": 12000000,
    "setMinUptimeScore": 10000000,
    "setMaxCounteroffersPerApplication": 10000000,
    "setMaxInvitesPerJob": 10000000,
    "setPaused": 10000000,
    "setOwner": 10000000,
}

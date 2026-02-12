use multiversx_sc::proxy_imports::*;
use shared_types::AcceptedOfferSummary;

pub struct JobBoardProxy;

impl<Env, From, To, Gas> TxProxyTrait<Env, From, To, Gas> for JobBoardProxy
where
    Env: TxEnv,
    From: TxFrom<Env>,
    To: TxTo<Env>,
    Gas: TxGas<Env>,
{
    type TxProxyMethods = JobBoardProxyMethods<Env, From, To, Gas>;

    fn proxy_methods(self, tx: Tx<Env, From, To, (), Gas, (), ()>) -> Self::TxProxyMethods {
        JobBoardProxyMethods { wrapped_tx: tx }
    }
}

pub struct JobBoardProxyMethods<Env, From, To, Gas>
where
    Env: TxEnv,
    From: TxFrom<Env>,
    To: TxTo<Env>,
    Gas: TxGas<Env>,
{
    wrapped_tx: Tx<Env, From, To, (), Gas, (), ()>,
}

impl<Env, From, To, Gas> JobBoardProxyMethods<Env, From, To, Gas>
where
    Env: TxEnv,
    Env::Api: VMApi,
    From: TxFrom<Env>,
    To: TxTo<Env>,
    Gas: TxGas<Env>,
{
    pub fn get_accepted_offer<Arg0: ProxyArg<u64>>(
        self,
        job_id: Arg0,
    ) -> TxTypedCall<Env, From, To, NotPayable, Gas, OptionalValue<AcceptedOfferSummary<Env::Api>>> {
        self.wrapped_tx
            .payment(NotPayable)
            .raw_call("getAcceptedOffer")
            .argument(&job_id)
            .original_result()
    }
}

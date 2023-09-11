use osmosis_test_tube::{
    fn_execute, fn_query,
    osmosis_std::types::osmosis::lockup::{
        AccountLockedCoinsRequest, AccountLockedCoinsResponse, MsgLockTokens, MsgLockTokensResponse,
    },
    Module, OsmosisTestApp, Runner,
};

pub struct Lockup<'a, R: Runner<'a>> {
    runner: &'a R,
}

impl<'a, R: Runner<'a>> Module<'a, R> for Lockup<'a, R> {
    fn new(runner: &'a R) -> Self {
        Self { runner }
    }
}

impl Lockup<'_, OsmosisTestApp> {
    fn_execute! {
        pub lock_tokens: MsgLockTokens => MsgLockTokensResponse
    }

    fn_query! {
        pub account_locked_coins ["/osmosis.lockup.Query/AccountLockedCoins"]: AccountLockedCoinsRequest => AccountLockedCoinsResponse
    }
}

# single-side-lp-cl-contract

This contract allows for a user to single-sided join a concentrated liquidity pool on Osmosis. We currently use a slightly naive approach to calculate the amount to swap, resulting in a small amount of dust funds after joining. This can be improved with a future contract, which should take into consideration liquidity depths surrounding the current tick. With this information, one should be able to determine exactly how much to swap to not leave any dust in the user's wallet after creating the position.

The contract must be called as a multi-message:

- User gives an Authz MsgGrant to the contract for these two message types:
  - `MsgSwapExactAmountIn` from PoolManager
  - `MsgCreatePosition` from ConcentratedLiquidity

- User calls the contract:

    ``` json
    {
    "single_sided_swap_and_lp": {
        "pool_id": 1,
        "lower_tick": -10800,
        "upper_tick": 342000000,
        "token_provided": {
        "amount": "100000",
        "denom": "token0"
        },
        "token_min_amount0": "0",
        "token_min_amount1": "0"
    }
    }
    ```

- User revokes the Authz MsgGrant to the contract for the two message types

The flow of the contract is as follows:
    1. User calls the `single_sided_swap_and_lp` exec message
    2. The contract swaps the provided token on the user's behalf for the other token in the pool at a ratio that facilitates the creation of a position at the provided tick range
    3. The contract creates a position on the user's behalf with the swapped token and the remaining provided token

use cosmwasm_schema::write_api;

use single_sided_swap_cl::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};

fn main() {
    write_api! {
        name: "osmosis-single-sided-swap",
        instantiate: InstantiateMsg,
        query: QueryMsg,
        execute: ExecuteMsg,
    };
}

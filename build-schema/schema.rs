use cosmwasm_schema::write_api;

use single_sided_lp_cl::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};

fn main() {
    write_api! {
        name: "osmosis-single-sided-lp-cl",
        instantiate: InstantiateMsg,
        query: QueryMsg,
        execute: ExecuteMsg,
    };
}

#![forbid(unsafe_code)]

mod contracts;
mod fcg;
mod io;
mod risk;
mod slm;

use seraph_types::{Knowledge, Models};

pub fn build_models_from_knowledge(knowledge: &Knowledge) -> Result<Models, String> {
    let fcg = fcg::build_fcg(knowledge);
    let slm = slm::build_slm_models(knowledge);
    let api_contracts = contracts::build_api_contracts(knowledge);
    let risk_surface_map = risk::build_risk_surface_map(knowledge, &api_contracts);

    Ok(Models {
        fcg,
        slm,
        api_contracts,
        risk_surface_map,
    })
}

pub use io::{read_knowledge_json, write_models_json};

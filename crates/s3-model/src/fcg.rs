use seraph_types::FunctionalCapabilityGraph;
use seraph_types::Knowledge;
use std::collections::BTreeMap;

pub fn build_fcg(_knowledge: &Knowledge) -> FunctionalCapabilityGraph {
    FunctionalCapabilityGraph {
        capabilities: Vec::new(),
        capability_chains: Vec::new(),
        capability_api_index: BTreeMap::new(),
        stage1_summary: String::new(),
    }
}

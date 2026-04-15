use seraph_types::{ApiContract, Knowledge, RiskSurfaceMap, TypeSynthesisOverview};
use std::collections::BTreeMap;

pub fn build_risk_surface_map(
    _knowledge: &Knowledge,
    _api_contracts: &[ApiContract],
) -> RiskSurfaceMap {
    RiskSurfaceMap {
        api_risks: Vec::new(),
        type_synthesis_overview: TypeSynthesisOverview {
            generic_api_count: 0,
            strategy_distribution: BTreeMap::new(),
            one_liner: String::new(),
        },
        rust_feature_risks: Vec::new(),
    }
}

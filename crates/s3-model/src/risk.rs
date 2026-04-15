use seraph_types::{
    ApiContract, ApiId, ApiRisk, Knowledge, ReprKind, RiskLevel, RiskSurfaceMap,
    RustFeatureRisk, TypeSynthesisOverview,
};
use std::collections::BTreeMap;

pub fn build_risk_surface_map(
    knowledge: &Knowledge,
    api_contracts: &[ApiContract],
) -> RiskSurfaceMap {
    let api_risks = api_contracts
        .iter()
        .map(|contract| build_api_risk(knowledge, contract))
        .collect::<Vec<_>>();

    RiskSurfaceMap {
        api_risks,
        type_synthesis_overview: build_type_synthesis_overview(api_contracts),
        rust_feature_risks: build_rust_feature_risks(knowledge),
    }
}

fn build_api_risk(knowledge: &Knowledge, contract: &ApiContract) -> ApiRisk {
    let mut score = 0;
    let mut reasons = Vec::new();

    if contract
        .side_effects
        .iter()
        .any(|effect| effect == "consumes input bytes")
    {
        score += 1;
        reasons.push("external input surface".to_owned());
    }
    if contract
        .generic_constraints
        .as_ref()
        .map(|constraints| constraints.params.iter().any(|param| param.strategy == "C"))
        .unwrap_or(false)
    {
        score += 2;
        reasons.push("generic synthesis strategy C".to_owned());
    }
    if knowledge
        .risk_facts
        .extern_abi_apis
        .iter()
        .any(|fact| fact.api_id == contract.api_id)
    {
        score += 3;
        reasons.push("extern ABI boundary".to_owned());
    }
    if !contract.error_conditions.is_empty() {
        score += 1;
        reasons.push("recoverable error path".to_owned());
    }
    if !contract.panic_conditions.is_empty() {
        score += 1;
        reasons.push("documented panic conditions".to_owned());
    }
    if knowledge
        .risk_facts
        .borrowed_return_apis
        .iter()
        .any(|fact| fact.api_id == contract.api_id)
    {
        score += 1;
        reasons.push("borrowed return ties output to input".to_owned());
    }

    let risk_level = match score {
        0 | 1 => RiskLevel::Low,
        2 | 3 => RiskLevel::Medium,
        _ => RiskLevel::High,
    };

    ApiRisk {
        api_id: contract.api_id.clone(),
        risk_level,
        reasons,
        recommended_fuzz_strategy: recommend_fuzz_strategy(contract, knowledge),
    }
}

fn build_type_synthesis_overview(api_contracts: &[ApiContract]) -> TypeSynthesisOverview {
    let mut strategy_distribution = BTreeMap::<String, u32>::new();
    let mut generic_api_count = 0;

    for contract in api_contracts {
        let Some(generic_constraints) = &contract.generic_constraints else {
            continue;
        };
        generic_api_count += 1;
        for param in &generic_constraints.params {
            *strategy_distribution
                .entry(param.strategy.clone())
                .or_insert(0) += 1;
        }
    }

    let c_count = strategy_distribution.get("C").copied().unwrap_or(0);
    let one_liner =
        format!("{generic_api_count} 个泛型 API，其中 {c_count} 个适合自定义类型合成");

    TypeSynthesisOverview {
        generic_api_count,
        strategy_distribution,
        one_liner,
    }
}

fn build_rust_feature_risks(knowledge: &Knowledge) -> Vec<RustFeatureRisk> {
    let mut risks = Vec::new();

    let extern_abi_apis = knowledge
        .risk_facts
        .extern_abi_apis
        .iter()
        .map(|fact| fact.api_id.clone())
        .collect::<Vec<_>>();
    if !extern_abi_apis.is_empty() {
        risks.push(RustFeatureRisk {
            feature: "extern_abi".to_owned(),
            apis_affected: extern_abi_apis,
            risk: "public API crosses a non-Rust ABI boundary".to_owned(),
        });
    }

    let borrowed_return_apis = knowledge
        .risk_facts
        .borrowed_return_apis
        .iter()
        .map(|fact| fact.api_id.clone())
        .collect::<Vec<_>>();
    if !borrowed_return_apis.is_empty() {
        risks.push(RustFeatureRisk {
            feature: "borrowed_return".to_owned(),
            apis_affected: borrowed_return_apis,
            risk: "returned value borrows from self or input arguments".to_owned(),
        });
    }

    let repr_packed_apis = knowledge
        .risk_facts
        .repr_types
        .iter()
        .filter(|fact| fact.repr_kinds.iter().any(|kind| matches!(kind, ReprKind::Packed)))
        .flat_map(|_| Vec::<ApiId>::new())
        .collect::<Vec<_>>();
    if !repr_packed_apis.is_empty() {
        risks.push(RustFeatureRisk {
            feature: "repr_packed".to_owned(),
            apis_affected: repr_packed_apis,
            risk: "packed repr can make references invalid or surprising".to_owned(),
        });
    }

    risks
}

fn recommend_fuzz_strategy(contract: &ApiContract, knowledge: &Knowledge) -> String {
    if knowledge
        .risk_facts
        .extern_abi_apis
        .iter()
        .any(|fact| fact.api_id == contract.api_id)
    {
        "ffi boundary inputs".to_owned()
    } else if contract
        .generic_constraints
        .as_ref()
        .map(|constraints| constraints.params.iter().any(|param| param.strategy == "C"))
        .unwrap_or(false)
    {
        "raw bytes + custom reader".to_owned()
    } else if knowledge
        .risk_facts
        .borrowed_return_apis
        .iter()
        .any(|fact| fact.api_id == contract.api_id)
    {
        "reuse owner and borrowed output across state changes".to_owned()
    } else {
        "exercise documented public entry points".to_owned()
    }
}

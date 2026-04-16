use seraph_types::{
    ApiId, ApiInfo, ApiKind, CapId, CapabilityAnchorKind, CapabilityNode, CapabilityRole,
    FunctionalCapabilityGraph, Knowledge, ModuleId, Stage1CapabilityCard, Stage1Summary, TypeId,
};
use std::collections::{BTreeMap, BTreeSet};

pub fn build_fcg(knowledge: &Knowledge) -> FunctionalCapabilityGraph {
    let module_index = knowledge
        .modules
        .iter()
        .map(|module| (module.module_id.clone(), module))
        .collect::<BTreeMap<_, _>>();
    let type_index = knowledge
        .types
        .iter()
        .map(|ty| (ty.type_id.clone(), ty))
        .collect::<BTreeMap<_, _>>();
    let api_index = knowledge
        .apis
        .iter()
        .map(|api| (api.api_id.clone(), api))
        .collect::<BTreeMap<_, _>>();
    let type_lookup = build_type_lookup(knowledge);

    let mut grouped = BTreeMap::<CapId, CapabilityDraft>::new();
    for api in &knowledge.apis {
        let anchor = determine_anchor(api, &module_index, &type_index);
        let role = classify_role(api, knowledge, &type_lookup);
        let cap_id = capability_id(anchor.anchor_path.as_str(), anchor.anchor_kind, role);

        let draft = grouped.entry(cap_id.clone()).or_insert_with(|| CapabilityDraft {
            cap_id: cap_id.clone(),
            anchor_kind: anchor.anchor_kind,
            anchor_module_id: anchor.anchor_module_id.clone(),
            anchor_type_id: anchor.anchor_type_id.clone(),
            anchor_path: anchor.anchor_path.clone(),
            role,
            name: capability_name(anchor.anchor_kind, anchor.anchor_path.as_str(), role),
            description: capability_description(anchor.summary.as_str(), role),
            api_ids: Vec::new(),
            entry_api_ids: Vec::new(),
            connects_to: Vec::new(),
        });

        draft.api_ids.push(api.api_id.clone());
        if is_entry_api(api, role) {
            draft.entry_api_ids.push(api.api_id.clone());
        }
    }

    let mut capabilities = grouped.into_values().collect::<Vec<_>>();

    let caps_by_type = capabilities
        .iter()
        .filter_map(|cap| {
            cap.anchor_type_id
                .as_ref()
                .map(|type_id| (type_id.clone(), (cap.role, cap.cap_id.clone())))
        })
        .fold(
            BTreeMap::<TypeId, Vec<(CapabilityRole, CapId)>>::new(),
            |mut acc, (type_id, entry)| {
                acc.entry(type_id).or_default().push(entry);
                acc
            },
        );
    let caps_by_module_entry = capabilities
        .iter()
        .filter(|cap| matches!(cap.anchor_kind, CapabilityAnchorKind::ModuleEntry))
        .fold(
            BTreeMap::<ModuleId, Vec<(CapabilityRole, CapId)>>::new(),
            |mut acc, cap| {
                acc.entry(cap.anchor_module_id.clone())
                    .or_default()
                    .push((cap.role, cap.cap_id.clone()));
                acc
            },
        );

    for capability in &mut capabilities {
        let mut targets = BTreeSet::new();

        if capability.role == CapabilityRole::Construction {
            match &capability.anchor_type_id {
                Some(type_id) => {
                    if let Some(entries) = caps_by_type.get(type_id) {
                        targets.extend(construction_followups(entries));
                    }
                }
                None => {
                    if let Some(entries) = caps_by_module_entry.get(&capability.anchor_module_id) {
                        targets.extend(construction_followups(entries));
                    }
                }
            }
        }

        for api_id in &capability.api_ids {
            if !role_allows_cross_anchor(capability.role) {
                continue;
            }
            let Some(api) = api_index.get(api_id) else {
                continue;
            };

            for candidate in candidate_return_types(api) {
                let Some(target_type_id) = resolve_type_id(candidate.as_str(), &type_lookup) else {
                    continue;
                };
                if capability.anchor_type_id.as_ref() == Some(&target_type_id) {
                    continue;
                }
                if let Some(target_cap_id) = preferred_type_capability(&target_type_id, &caps_by_type)
                {
                    if target_cap_id != capability.cap_id {
                        targets.insert(target_cap_id);
                    }
                }
            }
        }

        capability.connects_to = targets.into_iter().collect();
    }

    let capability_api_index = capabilities
        .iter()
        .map(|cap| (cap.cap_id.clone(), cap.api_ids.clone()))
        .collect::<BTreeMap<_, _>>();

    let capability_nodes = capabilities
        .iter()
        .map(|cap| CapabilityNode {
            cap_id: cap.cap_id.clone(),
            anchor_kind: cap.anchor_kind,
            anchor_module_id: cap.anchor_module_id.clone(),
            anchor_type_id: cap.anchor_type_id.clone(),
            role: cap.role,
            name: cap.name.clone(),
            description: cap.description.clone(),
            api_ids: cap.api_ids.clone(),
            entry_api_ids: cap.entry_api_ids.clone(),
            connects_to: cap.connects_to.clone(),
        })
        .collect::<Vec<_>>();
    let capability_chains = derive_capability_chains(&capability_nodes);
    let stage1_summary = build_stage1_summary(&capabilities, &capability_chains);

    FunctionalCapabilityGraph {
        capabilities: capability_nodes,
        capability_chains,
        capability_api_index,
        stage1_summary,
    }
}

#[derive(Clone)]
struct CapabilityDraft {
    cap_id: CapId,
    anchor_kind: CapabilityAnchorKind,
    anchor_module_id: ModuleId,
    anchor_type_id: Option<TypeId>,
    anchor_path: String,
    role: CapabilityRole,
    name: String,
    description: String,
    api_ids: Vec<ApiId>,
    entry_api_ids: Vec<ApiId>,
    connects_to: Vec<CapId>,
}

struct AnchorInfo {
    anchor_kind: CapabilityAnchorKind,
    anchor_module_id: ModuleId,
    anchor_type_id: Option<TypeId>,
    anchor_path: String,
    summary: String,
}

fn determine_anchor<'a>(
    api: &ApiInfo,
    module_index: &BTreeMap<ModuleId, &'a seraph_types::ModuleInfo>,
    type_index: &BTreeMap<TypeId, &'a seraph_types::TypeInfo>,
) -> AnchorInfo {
    if let Some(owner_type_id) = &api.owner_type_id {
        if let Some(owner_type) = type_index.get(owner_type_id) {
            return AnchorInfo {
                anchor_kind: CapabilityAnchorKind::Type,
                anchor_module_id: owner_type.public_anchor_module_id.clone(),
                anchor_type_id: Some(owner_type.type_id.clone()),
                anchor_path: owner_type.canonical_path.clone(),
                summary: owner_type.doc_sections.summary.clone(),
            };
        }
    }

    let anchor_path = module_index
        .get(&api.public_anchor_module_id)
        .map(|module| module.canonical_path.clone())
        .unwrap_or_else(|| api.public_anchor_module_id.as_str().replacen("mod::", "", 1));
    let summary = module_index
        .get(&api.public_anchor_module_id)
        .map(|module| module.doc_sections.summary.clone())
        .unwrap_or_default();

    AnchorInfo {
        anchor_kind: CapabilityAnchorKind::ModuleEntry,
        anchor_module_id: api.public_anchor_module_id.clone(),
        anchor_type_id: None,
        anchor_path,
        summary,
    }
}

fn classify_role(api: &ApiInfo, knowledge: &Knowledge, type_lookup: &TypeLookup) -> CapabilityRole {
    if knowledge
        .risk_facts
        .extern_abi_apis
        .iter()
        .any(|fact| fact.api_id == api.api_id)
    {
        return CapabilityRole::Ffi;
    }

    if is_finalization_like(api.name.as_str()) {
        return CapabilityRole::Finalization;
    }
    if is_iteration_like(api) {
        return CapabilityRole::Iteration;
    }
    if is_conversion_like(api) {
        return CapabilityRole::Conversion;
    }
    if is_construction_api(api, type_lookup) {
        return CapabilityRole::Construction;
    }
    if receiver_is_mut(api.receiver.as_deref()) {
        return CapabilityRole::Mutation;
    }

    CapabilityRole::Query
}

fn is_construction_api(api: &ApiInfo, type_lookup: &TypeLookup) -> bool {
    if matches!(api.api_kind, ApiKind::Constructor) {
        return true;
    }

    if matches!(api.api_kind, ApiKind::FreeFunction | ApiKind::AssocFunction) {
        if is_construction_like_name(api.name.as_str()) {
            return true;
        }
        return candidate_return_types(api)
            .iter()
            .any(|candidate| resolve_type_id(candidate.as_str(), type_lookup).is_some());
    }

    false
}

fn is_construction_like_name(name: &str) -> bool {
    name == "new"
        || name.starts_with("from_")
        || name.starts_with("with_")
        || matches!(name, "parse" | "load" | "open" | "connect" | "begin")
}

fn is_entry_api(api: &ApiInfo, role: CapabilityRole) -> bool {
    match role {
        CapabilityRole::Construction | CapabilityRole::Ffi => matches!(
            api.api_kind,
            ApiKind::FreeFunction | ApiKind::AssocFunction | ApiKind::Constructor
        ),
        _ => false,
    }
}

fn capability_id(
    anchor_path: &str,
    anchor_kind: CapabilityAnchorKind,
    role: CapabilityRole,
) -> CapId {
    match anchor_kind {
        CapabilityAnchorKind::ModuleEntry => CapId::from(format!(
            "cap::{anchor_path}::module_entry::{}",
            role_slug(role)
        )),
        CapabilityAnchorKind::Type => {
            CapId::from(format!("cap::{anchor_path}::{}", role_slug(role)))
        }
    }
}

fn capability_name(anchor_kind: CapabilityAnchorKind, anchor_path: &str, role: CapabilityRole) -> String {
    match anchor_kind {
        CapabilityAnchorKind::ModuleEntry => {
            format!("{} 入口{}", short_name(anchor_path), role_label(role))
        }
        CapabilityAnchorKind::Type => format!("{} {}", short_name(anchor_path), role_label(role)),
    }
}

fn capability_description(summary: &str, role: CapabilityRole) -> String {
    let trimmed = summary.trim();
    if trimmed.is_empty() {
        role_description(role).to_owned()
    } else {
        format!("{trimmed}：{}", role_description(role))
    }
}

fn role_label(role: CapabilityRole) -> &'static str {
    match role {
        CapabilityRole::Construction => "构造",
        CapabilityRole::Query => "查询",
        CapabilityRole::Mutation => "修改",
        CapabilityRole::Iteration => "迭代",
        CapabilityRole::Conversion => "转换",
        CapabilityRole::Finalization => "终结",
        CapabilityRole::Ffi => "FFI",
    }
}

fn role_description(role: CapabilityRole) -> &'static str {
    match role {
        CapabilityRole::Construction => "创建对象或进入主流程的入口 API",
        CapabilityRole::Query => "围绕对象的只读访问与检查",
        CapabilityRole::Mutation => "围绕对象的可变更新与状态变化",
        CapabilityRole::Iteration => "返回迭代器或遍历视图",
        CapabilityRole::Conversion => "在语义面之间执行转换",
        CapabilityRole::Finalization => "将对象推进到终态或关闭流程",
        CapabilityRole::Ffi => "显式 ABI / FFI 边界相关 API",
    }
}

fn role_slug(role: CapabilityRole) -> &'static str {
    match role {
        CapabilityRole::Construction => "construction",
        CapabilityRole::Query => "query",
        CapabilityRole::Mutation => "mutation",
        CapabilityRole::Iteration => "iteration",
        CapabilityRole::Conversion => "conversion",
        CapabilityRole::Finalization => "finalization",
        CapabilityRole::Ffi => "ffi",
    }
}

fn construction_followups(entries: &[(CapabilityRole, CapId)]) -> Vec<CapId> {
    let order = [
        CapabilityRole::Query,
        CapabilityRole::Mutation,
        CapabilityRole::Iteration,
        CapabilityRole::Conversion,
        CapabilityRole::Finalization,
    ];

    order
        .iter()
        .filter_map(|role| {
            entries
                .iter()
                .find(|(entry_role, _)| entry_role == role)
                .map(|(_, cap_id)| cap_id.clone())
        })
        .collect()
}

fn preferred_type_capability(
    target_type_id: &TypeId,
    caps_by_type: &BTreeMap<TypeId, Vec<(CapabilityRole, CapId)>>,
) -> Option<CapId> {
    let entries = caps_by_type.get(target_type_id)?;
    let preferred_order = [
        CapabilityRole::Query,
        CapabilityRole::Mutation,
        CapabilityRole::Iteration,
        CapabilityRole::Conversion,
        CapabilityRole::Finalization,
        CapabilityRole::Construction,
        CapabilityRole::Ffi,
    ];

    preferred_order.iter().find_map(|role| {
        entries
            .iter()
            .find(|(entry_role, _)| entry_role == role)
            .map(|(_, cap_id)| cap_id.clone())
    })
}

fn role_allows_cross_anchor(role: CapabilityRole) -> bool {
    matches!(role, CapabilityRole::Construction | CapabilityRole::Conversion)
}

fn candidate_return_types(api: &ApiInfo) -> Vec<String> {
    let mut candidates = Vec::new();
    if let Some(return_type) = &api.return_type {
        candidates.push(return_type.clone());
    }
    if let Some(return_shape) = &api.return_shape {
        candidates.extend(return_shape.inner_types.iter().cloned());
    }
    candidates
}

struct TypeLookup {
    exact_paths: BTreeMap<String, TypeId>,
    unique_short_names: BTreeMap<String, TypeId>,
}

fn build_type_lookup(knowledge: &Knowledge) -> TypeLookup {
    let mut exact_paths = BTreeMap::new();
    let mut short_name_candidates = BTreeMap::<String, Option<TypeId>>::new();

    for ty in &knowledge.types {
        exact_paths.insert(ty.canonical_path.clone(), ty.type_id.clone());
        record_unique_short_name(
            &mut short_name_candidates,
            short_name(ty.canonical_path.as_str()),
            ty.type_id.clone(),
        );

        for public_path in &ty.public_paths {
            exact_paths.insert(public_path.clone(), ty.type_id.clone());
            record_unique_short_name(
                &mut short_name_candidates,
                short_name(public_path.as_str()),
                ty.type_id.clone(),
            );
        }
    }

    let unique_short_names = short_name_candidates
        .into_iter()
        .filter_map(|(name, type_id)| type_id.map(|type_id| (name, type_id)))
        .collect();

    TypeLookup {
        exact_paths,
        unique_short_names,
    }
}

fn resolve_type_id(candidate_type: &str, type_lookup: &TypeLookup) -> Option<TypeId> {
    if let Some(type_id) = type_lookup.exact_paths.get(candidate_type) {
        return Some(type_id.clone());
    }

    let normalized = normalize_type_text(candidate_type);
    if normalized.is_empty() {
        return None;
    }

    if let Some(type_id) = type_lookup.exact_paths.get(&normalized) {
        return Some(type_id.clone());
    }

    type_lookup
        .unique_short_names
        .get(short_name(normalized.as_str()))
        .cloned()
}

fn normalize_type_text(candidate_type: &str) -> String {
    let mut text = candidate_type.trim().to_owned();

    loop {
        let trimmed = text.trim_start();
        if let Some(rest) = trimmed.strip_prefix('&') {
            text = rest.trim_start().to_owned();
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("*const ") {
            text = rest.trim_start().to_owned();
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("*mut ") {
            text = rest.trim_start().to_owned();
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("dyn ") {
            text = rest.trim_start().to_owned();
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("impl ") {
            text = rest.trim_start().to_owned();
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("mut ") {
            text = rest.trim_start().to_owned();
            continue;
        }
        if trimmed.starts_with('\'') {
            if let Some((_, rest)) = trimmed.split_once(' ') {
                text = rest.trim_start().to_owned();
                continue;
            }
        }
        text = trimmed.to_owned();
        break;
    }

    strip_generic_arguments(text.as_str()).trim().to_owned()
}

fn strip_generic_arguments(text: &str) -> String {
    let mut result = String::new();
    let mut depth = 0usize;

    for ch in text.chars() {
        match ch {
            '<' => depth += 1,
            '>' => depth = depth.saturating_sub(1),
            _ if depth == 0 => result.push(ch),
            _ => {}
        }
    }

    result
}

fn record_unique_short_name(
    candidates: &mut BTreeMap<String, Option<TypeId>>,
    short_name: &str,
    type_id: TypeId,
) {
    match candidates.get(short_name) {
        None => {
            candidates.insert(short_name.to_owned(), Some(type_id));
        }
        Some(Some(existing_type_id)) if *existing_type_id == type_id => {}
        Some(_) => {
            candidates.insert(short_name.to_owned(), None);
        }
    }
}

fn derive_capability_chains(capabilities: &[CapabilityNode]) -> Vec<Vec<CapId>> {
    let edges = capabilities
        .iter()
        .map(|cap| (cap.cap_id.clone(), cap.connects_to.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut indegree = BTreeMap::<CapId, usize>::new();
    for capability in capabilities {
        indegree.entry(capability.cap_id.clone()).or_insert(0);
        for target in &capability.connects_to {
            *indegree.entry(target.clone()).or_insert(0) += 1;
        }
    }

    let start_nodes = indegree
        .iter()
        .filter_map(|(cap_id, degree)| {
            if *degree == 0 {
                Some(cap_id.clone())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    let mut chains = BTreeSet::new();

    for start in start_nodes {
        let mut visited = BTreeSet::new();
        let mut path = Vec::new();
        dfs_collect_paths(&start, &edges, &mut visited, &mut path, &mut chains);
    }

    chains.into_iter().collect()
}

fn dfs_collect_paths(
    current: &CapId,
    edges: &BTreeMap<CapId, Vec<CapId>>,
    visited: &mut BTreeSet<CapId>,
    path: &mut Vec<CapId>,
    chains: &mut BTreeSet<Vec<CapId>>,
) {
    if !visited.insert(current.clone()) {
        return;
    }

    path.push(current.clone());
    let next = edges.get(current).cloned().unwrap_or_default();
    if next.is_empty() {
        if path.len() > 1 {
            chains.insert(path.clone());
        }
    } else {
        let mut traversed = false;
        for target in next {
            if visited.contains(&target) {
                continue;
            }
            traversed = true;
            dfs_collect_paths(&target, edges, visited, path, chains);
        }
        if !traversed && path.len() > 1 {
            chains.insert(path.clone());
        }
    }

    path.pop();
    visited.remove(current);
}

fn build_stage1_summary(
    capabilities: &[CapabilityDraft],
    chains: &[Vec<CapId>],
) -> Stage1Summary {
    let capability_cards = capabilities
        .iter()
        .map(|cap| Stage1CapabilityCard {
            cap_id: cap.cap_id.clone(),
            name: cap.name.clone(),
            anchor_path: cap.anchor_path.clone(),
            role: cap.role,
            description: cap.description.clone(),
        })
        .collect::<Vec<_>>();

    let one_liner = summarize_capabilities(capabilities, chains);

    Stage1Summary {
        capability_cards,
        recommended_chains: chains.to_vec(),
        one_liner,
    }
}

fn summarize_capabilities(capabilities: &[CapabilityDraft], chains: &[Vec<CapId>]) -> String {
    if capabilities.is_empty() {
        return "0项能力。".to_owned();
    }

    let featured = select_featured_capabilities(capabilities, chains, 3);
    let featured_names = featured
        .iter()
        .map(|cap| cap.name.as_str())
        .collect::<Vec<_>>()
        .join("、");

    if capabilities.len() <= featured.len() {
        format!("{}项能力：{}。", capabilities.len(), featured_names)
    } else {
        format!("{}项能力：{}等。", capabilities.len(), featured_names)
    }
}

fn select_featured_capabilities<'a>(
    capabilities: &'a [CapabilityDraft],
    chains: &[Vec<CapId>],
    limit: usize,
) -> Vec<&'a CapabilityDraft> {
    let chain_heads = chains
        .iter()
        .filter_map(|chain| chain.first().cloned())
        .collect::<BTreeSet<_>>();
    let chain_members = chains
        .iter()
        .flat_map(|chain| chain.iter().cloned())
        .collect::<BTreeSet<_>>();
    let mut ranked = capabilities.iter().collect::<Vec<_>>();
    ranked.sort_by(|left, right| {
        feature_rank(right, &chain_heads, &chain_members)
            .cmp(&feature_rank(left, &chain_heads, &chain_members))
            .then_with(|| left.name.cmp(&right.name))
    });
    ranked.into_iter().take(limit).collect()
}

fn feature_rank(
    capability: &CapabilityDraft,
    chain_heads: &BTreeSet<CapId>,
    chain_members: &BTreeSet<CapId>,
) -> (bool, bool, bool, usize, bool) {
    (
        !capability.entry_api_ids.is_empty(),
        chain_heads.contains(&capability.cap_id),
        matches!(capability.role, CapabilityRole::Ffi | CapabilityRole::Finalization),
        capability.api_ids.len(),
        chain_members.contains(&capability.cap_id),
    )
}

fn short_name(path: &str) -> &str {
    path.rsplit("::").next().unwrap_or(path)
}

fn receiver_is_mut(receiver: Option<&str>) -> bool {
    receiver
        .map(|receiver| {
            receiver
                .chars()
                .filter(|ch| !ch.is_whitespace())
                .collect::<String>()
                .to_ascii_lowercase()
                .contains("mutself")
        })
        .unwrap_or(false)
}

fn is_finalization_like(name: &str) -> bool {
    matches!(name, "close" | "finish" | "shutdown" | "end")
}

fn is_iteration_like(api: &ApiInfo) -> bool {
    matches!(api.name.as_str(), "iter" | "iter_mut" | "drain" | "cursor")
        || api
            .return_type
            .as_ref()
            .map(|ty| {
                ty.contains("Iter")
                    || ty.contains("Iterator")
                    || ty.contains("Drain")
                    || ty.contains("Cursor")
            })
            .unwrap_or(false)
}

fn is_conversion_like(api: &ApiInfo) -> bool {
    api.name.starts_with("into_") || api.name.starts_with("to_") || api.name.starts_with("as_")
}

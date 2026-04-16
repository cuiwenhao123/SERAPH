use seraph_types::{
    ApiInfo, ApiKind, CapId, CapabilityNode, FunctionalCapabilityGraph, Knowledge, ModuleId,
};
use std::collections::{BTreeMap, BTreeSet};

pub fn build_fcg(knowledge: &Knowledge) -> FunctionalCapabilityGraph {
    let api_index = group_apis_by_module(knowledge);
    let type_module_index = build_type_module_index(knowledge);

    let mut capabilities = knowledge
        .modules
        .iter()
        .filter_map(|module| {
            let apis = api_index.get(&module.module_id)?;
            let cap_id = capability_id_for_module(&module.canonical_path);
            let api_ids = apis
                .iter()
                .map(|api| api.api_id.clone())
                .collect::<Vec<_>>();
            let entry_api_ids = apis
                .iter()
                .filter(|api| {
                    matches!(
                        api.api_kind,
                        ApiKind::FreeFunction | ApiKind::AssocFunction | ApiKind::Constructor
                    )
                })
                .map(|api| api.api_id.clone())
                .collect::<Vec<_>>();

            Some(CapabilityNode {
                cap_id,
                name: pick_capability_name(
                    module.doc_sections.summary.as_str(),
                    module.name.as_str(),
                ),
                description: module.doc_sections.summary.trim().to_owned(),
                api_ids,
                entry_api_ids,
                connects_to: Vec::new(),
            })
        })
        .collect::<Vec<_>>();

    let cap_by_module = capabilities
        .iter()
        .map(|cap| {
            (
                ModuleId(cap.cap_id.as_str().replacen("cap::", "mod::", 1)),
                cap.cap_id.clone(),
            )
        })
        .collect::<BTreeMap<_, _>>();

    for capability in &mut capabilities {
        let module_id = ModuleId(capability.cap_id.as_str().replacen("cap::", "mod::", 1));
        if let Some(apis) = api_index.get(&module_id) {
            capability.connects_to =
                collect_connects_to(apis, &module_id, &type_module_index, &cap_by_module);
        }
    }

    let capability_api_index = capabilities
        .iter()
        .map(|cap| (cap.cap_id.clone(), cap.api_ids.clone()))
        .collect::<BTreeMap<_, _>>();
    let capability_chains = derive_capability_chains(&capabilities);
    let stage1_summary = build_stage1_summary(&capabilities, &capability_chains);

    FunctionalCapabilityGraph {
        capabilities,
        capability_chains,
        capability_api_index,
        stage1_summary,
    }
}

fn group_apis_by_module<'a>(knowledge: &'a Knowledge) -> BTreeMap<ModuleId, Vec<&'a ApiInfo>> {
    let mut grouped = BTreeMap::<ModuleId, Vec<&ApiInfo>>::new();
    for api in &knowledge.apis {
        grouped
            .entry(api.public_anchor_module_id.clone())
            .or_default()
            .push(api);
    }
    grouped
}

struct TypeModuleIndex {
    exact_paths: BTreeMap<String, ModuleId>,
    unique_short_names: BTreeMap<String, ModuleId>,
}

fn build_type_module_index(knowledge: &Knowledge) -> TypeModuleIndex {
    let mut exact_paths = BTreeMap::new();
    let mut short_name_candidates = BTreeMap::<String, Option<ModuleId>>::new();

    for ty in &knowledge.types {
        exact_paths.insert(
            ty.canonical_path.clone(),
            ty.public_anchor_module_id.clone(),
        );
        record_unique_short_name(
            &mut short_name_candidates,
            short_name(ty.canonical_path.as_str()),
            ty.public_anchor_module_id.clone(),
        );
        for public_path in &ty.public_paths {
            exact_paths.insert(public_path.clone(), ty.public_anchor_module_id.clone());
            record_unique_short_name(
                &mut short_name_candidates,
                short_name(public_path.as_str()),
                ty.public_anchor_module_id.clone(),
            );
        }
    }

    let unique_short_names = short_name_candidates
        .into_iter()
        .filter_map(|(name, module_id)| module_id.map(|module_id| (name, module_id)))
        .collect();

    TypeModuleIndex {
        exact_paths,
        unique_short_names,
    }
}

fn capability_id_for_module(module_path: &str) -> CapId {
    CapId::from(format!("cap::{module_path}"))
}

fn pick_capability_name(summary: &str, fallback: &str) -> String {
    let trimmed = summary.trim();
    if trimmed.is_empty() {
        fallback.to_owned()
    } else {
        trimmed.to_owned()
    }
}

fn collect_connects_to(
    apis: &[&ApiInfo],
    source_module_id: &ModuleId,
    type_module_index: &TypeModuleIndex,
    cap_by_module: &BTreeMap<ModuleId, CapId>,
) -> Vec<CapId> {
    let mut targets = BTreeSet::new();

    for api in apis {
        let mut candidate_types = Vec::new();
        if let Some(return_type) = &api.return_type {
            candidate_types.push(return_type.clone());
        }
        if let Some(return_shape) = &api.return_shape {
            candidate_types.extend(return_shape.inner_types.iter().cloned());
        }

        for candidate_type in candidate_types {
            let Some(target_module_id) = resolve_type_module(&candidate_type, type_module_index)
            else {
                continue;
            };
            if &target_module_id == source_module_id {
                continue;
            }
            if let Some(cap_id) = cap_by_module.get(&target_module_id) {
                targets.insert(cap_id.clone());
            }
        }
    }

    targets.into_iter().collect()
}

fn resolve_type_module(
    candidate_type: &str,
    type_module_index: &TypeModuleIndex,
) -> Option<ModuleId> {
    if let Some(module_id) = type_module_index.exact_paths.get(candidate_type) {
        return Some(module_id.clone());
    }

    let normalized = normalize_type_text(candidate_type);
    if normalized.is_empty() {
        return None;
    }

    if let Some(module_id) = type_module_index.exact_paths.get(&normalized) {
        return Some(module_id.clone());
    }

    type_module_index
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

fn short_name(path: &str) -> &str {
    path.rsplit("::").next().unwrap_or(path)
}

fn record_unique_short_name(
    candidates: &mut BTreeMap<String, Option<ModuleId>>,
    short_name: &str,
    module_id: ModuleId,
) {
    match candidates.get(short_name) {
        None => {
            candidates.insert(short_name.to_owned(), Some(module_id));
        }
        Some(Some(existing_module_id)) if *existing_module_id == module_id => {}
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

fn build_stage1_summary(capabilities: &[CapabilityNode], chains: &[Vec<CapId>]) -> String {
    let mut summary = format!("本库提供 {} 项核心能力", capabilities.len());
    if !capabilities.is_empty() {
        let capability_parts = capabilities
            .iter()
            .map(|cap| format!("{}（{} 个 API）", cap.name, cap.api_ids.len()))
            .collect::<Vec<_>>();
        summary.push('：');
        summary.push_str(&capability_parts.join("；"));
    }

    if let Some(first_chain) = chains.first() {
        summary.push_str("。典型能力链：");
        summary.push_str(
            &first_chain
                .iter()
                .filter_map(|cap_id| {
                    capabilities
                        .iter()
                        .find(|cap| &cap.cap_id == cap_id)
                        .map(|cap| cap.name.as_str())
                })
                .collect::<Vec<_>>()
                .join("→"),
        );
    }

    summary
}

use seraph_types::{
    ApiContract, ApiInfo, AssociatedTypeConstraint, BugHuntingValue, GenericConstraintParam,
    GenericConstraints, Knowledge, TraitId, TraitInfo, TypeId, TypeInfo,
};
use std::collections::{BTreeMap, BTreeSet};

pub fn build_api_contracts(knowledge: &Knowledge) -> Vec<ApiContract> {
    let trait_index = TraitIndex::new(knowledge);
    let type_index = TypeIndex::new(knowledge);

    knowledge
        .apis
        .iter()
        .map(|api| {
            let generic_constraints =
                build_generic_constraints(knowledge, api, &trait_index, &type_index);

            ApiContract {
                api_id: api.api_id.clone(),
                path: api.canonical_path.clone(),
                preconditions: derive_preconditions(api),
                postconditions: derive_postconditions(api),
                panic_conditions: split_non_empty_lines(&api.doc_sections.panics),
                error_conditions: split_non_empty_lines(&api.doc_sections.errors),
                safety: non_empty(api.doc_sections.safety.trim()),
                side_effects: derive_side_effects(knowledge, api, generic_constraints.as_ref()),
                generic_constraints,
            }
        })
        .collect()
}

fn derive_preconditions(api: &ApiInfo) -> Vec<String> {
    let mut preconditions = split_non_empty_lines(&api.doc_sections.safety);

    preconditions.extend(
        split_non_empty_lines(&api.doc_sections.panics)
            .into_iter()
            .filter(|line| line.to_ascii_lowercase().contains("if ")),
    );

    dedup_vec(preconditions)
}

fn derive_postconditions(api: &ApiInfo) -> Vec<String> {
    if let Some(return_type) = &api.return_type {
        vec![format!("returns {return_type}")]
    } else {
        Vec::new()
    }
}

fn derive_side_effects(
    knowledge: &Knowledge,
    api: &ApiInfo,
    generic_constraints: Option<&GenericConstraints>,
) -> Vec<String> {
    let mut side_effects = Vec::new();

    match classify_receiver(api.receiver.as_deref()) {
        ReceiverKind::Mutates => side_effects.push("mutates receiver".to_owned()),
        ReceiverKind::Consumes => side_effects.push("consumes receiver".to_owned()),
        ReceiverKind::Other => {}
    }

    if generic_constraints
        .map(|constraints| {
            constraints.params.iter().any(|param| {
                param
                    .full_bound_chain
                    .iter()
                    .any(|bound| is_reader_like(bound.as_str()))
            })
        })
        .unwrap_or(false)
        || api.arg_types.iter().any(|arg| arg == "&[u8]")
    {
        side_effects.push("consumes input bytes".to_owned());
    }

    if knowledge
        .risk_facts
        .extern_abi_apis
        .iter()
        .any(|fact| fact.api_id == api.api_id)
    {
        side_effects.push("crosses extern ABI boundary".to_owned());
    }

    dedup_vec(side_effects)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReceiverKind {
    Mutates,
    Consumes,
    Other,
}

fn classify_receiver(receiver: Option<&str>) -> ReceiverKind {
    let Some(receiver) = receiver else {
        return ReceiverKind::Other;
    };
    let normalized = receiver
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<String>()
        .to_ascii_lowercase();

    if normalized == "self" {
        ReceiverKind::Consumes
    } else if normalized.contains("mutself") {
        ReceiverKind::Mutates
    } else {
        ReceiverKind::Other
    }
}

fn build_generic_constraints(
    knowledge: &Knowledge,
    api: &ApiInfo,
    trait_index: &TraitIndex<'_>,
    type_index: &TypeIndex<'_>,
) -> Option<GenericConstraints> {
    let (generic_params, where_clauses) = collect_generic_context(knowledge, api, type_index);
    if generic_params.is_empty() {
        return None;
    }

    Some(GenericConstraints {
        params: generic_params
            .iter()
            .map(|param| build_generic_param(param.as_str(), &where_clauses, trait_index))
            .collect(),
    })
}

fn collect_generic_context(
    knowledge: &Knowledge,
    api: &ApiInfo,
    type_index: &TypeIndex<'_>,
) -> (Vec<String>, Vec<String>) {
    let mut generic_params = Vec::new();
    let mut where_clauses = Vec::new();

    if let Some(owner_type_id) = &api.owner_type_id {
        if let Some(owner_type) = type_index.resolve(owner_type_id) {
            generic_params.extend(owner_type.generic_params.iter().cloned());
            where_clauses.extend(owner_type.where_clauses.iter().cloned());
        } else if let Some(owner_type) = knowledge
            .types
            .iter()
            .find(|ty| &ty.type_id == owner_type_id)
        {
            generic_params.extend(owner_type.generic_params.iter().cloned());
            where_clauses.extend(owner_type.where_clauses.iter().cloned());
        }
    }

    generic_params.extend(api.generic_params.iter().cloned());
    where_clauses.extend(api.where_clauses.iter().cloned());

    (dedup_vec(generic_params), dedup_vec(where_clauses))
}

fn build_generic_param(
    param: &str,
    where_clauses: &[String],
    trait_index: &TraitIndex<'_>,
) -> GenericConstraintParam {
    let direct_bounds = where_clauses
        .iter()
        .filter_map(|clause| parse_direct_bounds(param, clause))
        .flatten()
        .collect::<Vec<_>>();
    let full_bound_chain = expand_full_bound_chain(&direct_bounds, trait_index);
    let associated_type_constraints =
        collect_associated_type_constraints(&direct_bounds, trait_index);
    let is_unsafe_trait = full_bound_chain.iter().any(|bound| {
        trait_index
            .resolve(bound.as_str())
            .map(|item| item.is_unsafe)
            .unwrap_or(false)
    });
    let strategy = choose_strategy(&full_bound_chain, is_unsafe_trait);
    let bug_hunting_value = choose_bug_hunting_value(strategy.as_str());
    let synthesis_guidance = choose_synthesis_guidance(&full_bound_chain, is_unsafe_trait);

    GenericConstraintParam {
        name: param.to_owned(),
        direct_bounds,
        full_bound_chain,
        associated_type_constraints,
        is_unsafe_trait,
        strategy,
        bug_hunting_value,
        synthesis_guidance,
    }
}

fn parse_direct_bounds(param: &str, clause: &str) -> Option<Vec<String>> {
    let (left, right) = clause.split_once(':')?;
    if left.trim() != param {
        return None;
    }

    Some(
        right
            .split('+')
            .map(str::trim)
            .filter(|bound| !bound.is_empty())
            .map(ToOwned::to_owned)
            .collect(),
    )
}

fn expand_full_bound_chain(direct_bounds: &[String], trait_index: &TraitIndex<'_>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut full = Vec::new();
    for direct_bound in direct_bounds {
        expand_bound_recursive(direct_bound.as_str(), trait_index, &mut seen, &mut full);
    }
    full
}

fn expand_bound_recursive(
    bound: &str,
    trait_index: &TraitIndex<'_>,
    seen: &mut BTreeSet<String>,
    full: &mut Vec<String>,
) {
    if !seen.insert(bound.to_owned()) {
        return;
    }

    full.push(bound.to_owned());
    if let Some(trait_info) = trait_index.resolve(bound) {
        for supertrait_id in &trait_info.direct_supertrait_ids {
            if let Some(supertrait) = trait_index.by_id.get(supertrait_id) {
                expand_bound_recursive(supertrait.canonical_path.as_str(), trait_index, seen, full);
            }
        }
    }
}

fn collect_associated_type_constraints(
    direct_bounds: &[String],
    trait_index: &TraitIndex<'_>,
) -> Vec<AssociatedTypeConstraint> {
    let mut constraints = Vec::new();
    for direct_bound in direct_bounds {
        let Some(trait_info) = trait_index.resolve(direct_bound.as_str()) else {
            continue;
        };
        for assoc_type in &trait_info.associated_type_defs {
            constraints.push(AssociatedTypeConstraint {
                trait_id: Some(trait_info.trait_id.clone()),
                trait_path: trait_info.canonical_path.clone(),
                associated_type_name: assoc_type.name.clone(),
                bounds: assoc_type.bounds.clone(),
            });
        }
    }
    constraints
}

fn choose_strategy(full_bound_chain: &[String], is_unsafe_trait: bool) -> String {
    if is_unsafe_trait {
        "D".to_owned()
    } else if full_bound_chain
        .iter()
        .any(|bound| is_reader_like(bound.as_str()) || bound.contains("Iterator"))
    {
        "C".to_owned()
    } else {
        "B".to_owned()
    }
}

fn choose_bug_hunting_value(strategy: &str) -> BugHuntingValue {
    match strategy {
        "C" => BugHuntingValue::High,
        "D" => BugHuntingValue::Medium,
        _ => BugHuntingValue::Medium,
    }
}

fn choose_synthesis_guidance(full_bound_chain: &[String], is_unsafe_trait: bool) -> String {
    if is_unsafe_trait {
        "prefer existing implementations only".to_owned()
    } else if full_bound_chain
        .iter()
        .any(|bound| is_reader_like(bound.as_str()))
    {
        "custom reader with short-read".to_owned()
    } else {
        "prefer known implementations".to_owned()
    }
}

fn is_reader_like(bound: &str) -> bool {
    let last_segment = bound.rsplit("::").next().unwrap_or(bound);
    matches!(last_segment, "Read" | "Reader" | "BufRead" | "Write")
}

fn split_non_empty_lines(section: &str) -> Vec<String> {
    section
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn non_empty(text: &str) -> Option<String> {
    if text.is_empty() {
        None
    } else {
        Some(text.to_owned())
    }
}

fn dedup_vec(values: Vec<String>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut deduped = Vec::new();
    for value in values {
        if seen.insert(value.clone()) {
            deduped.push(value);
        }
    }
    deduped
}

struct TraitIndex<'a> {
    by_id: BTreeMap<&'a TraitId, &'a TraitInfo>,
    by_path: BTreeMap<&'a str, &'a TraitInfo>,
    by_name: BTreeMap<&'a str, &'a TraitInfo>,
}

impl<'a> TraitIndex<'a> {
    fn new(knowledge: &'a Knowledge) -> Self {
        let mut by_id = BTreeMap::new();
        let mut by_path = BTreeMap::new();
        let mut by_name = BTreeMap::new();

        for trait_info in &knowledge.trait_registry {
            by_id.insert(&trait_info.trait_id, trait_info);
            by_path.insert(trait_info.canonical_path.as_str(), trait_info);
            by_name.insert(trait_info.name.as_str(), trait_info);
        }

        Self {
            by_id,
            by_path,
            by_name,
        }
    }

    fn resolve(&self, bound: &str) -> Option<&'a TraitInfo> {
        self.by_path.get(bound).copied().or_else(|| {
            self.by_name
                .get(bound.rsplit("::").next().unwrap_or(bound))
                .copied()
        })
    }
}

struct TypeIndex<'a> {
    by_id: BTreeMap<&'a TypeId, &'a TypeInfo>,
}

impl<'a> TypeIndex<'a> {
    fn new(knowledge: &'a Knowledge) -> Self {
        let mut by_id = BTreeMap::new();

        for type_info in &knowledge.types {
            by_id.insert(&type_info.type_id, type_info);
        }

        Self { by_id }
    }

    fn resolve(&self, type_id: &TypeId) -> Option<&'a TypeInfo> {
        self.by_id.get(type_id).copied()
    }
}

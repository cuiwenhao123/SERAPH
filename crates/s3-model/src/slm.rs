use seraph_types::{
    ApiInfo, ApiKind, ForbiddenTransition, Knowledge, SlmModelKind, StateLifecycleModel,
    StateTransition, TypeId,
};
use std::collections::BTreeSet;

pub fn build_slm_models(knowledge: &Knowledge) -> Vec<StateLifecycleModel> {
    let drop_types = knowledge
        .risk_facts
        .drop_impl_types
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();

    knowledge
        .types
        .iter()
        .filter_map(|ty| {
            let score = score_type(knowledge, &ty.type_id, &drop_types);
            if score == 0 {
                return None;
            }

            let mut model_kind = if score >= 3 {
                SlmModelKind::Full
            } else {
                SlmModelKind::Simplified
            };
            let related_apis = knowledge
                .apis
                .iter()
                .filter(|api| api.owner_type_id.as_ref() == Some(&ty.type_id))
                .collect::<Vec<_>>();
            let mut states = build_states(&related_apis, model_kind);
            let mut transitions = build_transitions(&related_apis, model_kind, &states);
            let forbidden_transitions = build_forbidden_transitions(&related_apis);

            if matches!(model_kind, SlmModelKind::Full)
                && !qualifies_for_full_model(&states, &transitions, &forbidden_transitions)
            {
                model_kind = SlmModelKind::Simplified;
                states = build_states(&related_apis, model_kind);
                transitions = build_transitions(&related_apis, model_kind, &states);
            }

            let fuzzable_states = choose_fuzzable_states(&states);

            Some(StateLifecycleModel {
                type_id: ty.type_id.clone(),
                path: ty.canonical_path.clone(),
                model_kind,
                states,
                transitions,
                fuzzable_states,
                forbidden_transitions,
            })
        })
        .collect()
}

fn qualifies_for_full_model(
    states: &[String],
    transitions: &[StateTransition],
    forbidden_transitions: &[ForbiddenTransition],
) -> bool {
    let has_nontrivial_state = states
        .iter()
        .any(|state| !matches!(state.as_str(), "Constructed" | "InUse"));
    let has_non_constructor_transition = transitions.iter().any(|transition| {
        !(transition.from == "Uninitialized" && transition.to == "Constructed")
            && !(transition.from == "Constructed" && transition.to == "Constructed")
    });

    has_nontrivial_state && (has_non_constructor_transition || !forbidden_transitions.is_empty())
}

fn score_type(knowledge: &Knowledge, type_id: &TypeId, drop_types: &BTreeSet<TypeId>) -> u32 {
    let related_apis = knowledge
        .apis
        .iter()
        .filter(|api| api.owner_type_id.as_ref() == Some(type_id))
        .collect::<Vec<_>>();
    let type_docs = knowledge
        .types
        .iter()
        .find(|ty| &ty.type_id == type_id)
        .map(|ty| format!("{} {}", ty.docs, ty.doc_sections.summary))
        .unwrap_or_default()
        .to_ascii_lowercase();

    let mut score = 0;
    if drop_types.contains(type_id) {
        score += 3;
    }
    if related_apis
        .iter()
        .filter(|api| is_mut_receiver(api.receiver.as_deref()))
        .count()
        >= 3
    {
        score += 2;
    }
    if contains_state_words(&type_docs) {
        score += 2;
    }
    if knowledge
        .apis
        .iter()
        .filter(|api| {
            api.return_shape
                .as_ref()
                .map(|shape| {
                    shape
                        .inner_types
                        .iter()
                        .any(|item| item == &find_type_path(knowledge, type_id))
                })
                .unwrap_or(false)
        })
        .count()
        > 1
    {
        score += 1;
    }
    if related_apis
        .iter()
        .any(|api| contains_state_words(&api.doc_sections.panics.to_ascii_lowercase()))
    {
        score += 1;
    }
    score
}

fn build_states(related_apis: &[&ApiInfo], model_kind: SlmModelKind) -> Vec<String> {
    let mut states = vec!["Constructed".to_owned()];

    if matches!(model_kind, SlmModelKind::Simplified) {
        if related_apis.iter().any(|api| api.receiver.is_some()) {
            states.push("InUse".to_owned());
        }
        if related_apis
            .iter()
            .any(|api| is_close_like(api.name.as_str()))
        {
            states.push("Closed".to_owned());
        }
        return states;
    }

    if related_apis
        .iter()
        .any(|api| !is_constructor_like(api) && is_configure_like(api.name.as_str()))
    {
        states.push("Configured".to_owned());
    }
    if related_apis
        .iter()
        .any(|api| !is_constructor_like(api) && is_activate_like(api.name.as_str()))
    {
        states.push("Active".to_owned());
    }
    if related_apis
        .iter()
        .any(|api| !is_constructor_like(api) && is_recovery_like(api.name.as_str()))
    {
        states.push("Error".to_owned());
    }
    if related_apis
        .iter()
        .any(|api| !is_constructor_like(api) && is_close_like(api.name.as_str()))
    {
        states.push("Closed".to_owned());
    }
    states
}

fn build_transitions(
    related_apis: &[&ApiInfo],
    model_kind: SlmModelKind,
    states: &[String],
) -> Vec<StateTransition> {
    let mut transitions = Vec::new();
    let has_configured_state = states.iter().any(|state| state == "Configured");
    let has_active_state = states.iter().any(|state| state == "Active");
    let has_error_state = states.iter().any(|state| state == "Error");

    for api in related_apis {
        if is_constructor_like(api) {
            transitions.push(StateTransition {
                from: "Uninitialized".into(),
                to: "Constructed".into(),
                via_api_id: api.api_id.clone(),
                preconditions: Vec::new(),
            });
            continue;
        }

        let (from, to) = if matches!(model_kind, SlmModelKind::Simplified) {
            if is_close_like(api.name.as_str()) {
                ("InUse", "Closed")
            } else {
                ("Constructed", "InUse")
            }
        } else if is_configure_like(api.name.as_str()) {
            ("Constructed", "Configured")
        } else if is_activate_like(api.name.as_str()) {
            if has_configured_state {
                ("Configured", "Active")
            } else {
                ("Constructed", "Active")
            }
        } else if is_recovery_like(api.name.as_str()) {
            if has_error_state {
                ("Error", "Constructed")
            } else {
                ("Constructed", "Constructed")
            }
        } else if is_close_like(api.name.as_str()) {
            if has_active_state {
                ("Active", "Closed")
            } else if has_configured_state {
                ("Configured", "Closed")
            } else {
                ("Constructed", "Closed")
            }
        } else {
            if has_active_state {
                ("Active", "Active")
            } else if has_configured_state {
                ("Configured", "Configured")
            } else {
                ("Constructed", "Constructed")
            }
        };

        transitions.push(StateTransition {
            from: from.into(),
            to: to.into(),
            via_api_id: api.api_id.clone(),
            preconditions: split_non_empty_lines(&api.doc_sections.safety),
        });
    }

    transitions
}

fn build_forbidden_transitions(related_apis: &[&ApiInfo]) -> Vec<ForbiddenTransition> {
    related_apis
        .iter()
        .filter_map(|api| {
            let reason = api.doc_sections.panics.trim();
            if reason.is_empty() {
                return None;
            }

            Some(ForbiddenTransition {
                from: infer_forbidden_from_state(reason),
                via_api_id: api.api_id.clone(),
                reason: reason.to_owned(),
            })
        })
        .collect()
}

fn choose_fuzzable_states(states: &[String]) -> Vec<String> {
    if states.iter().any(|state| state == "Active") {
        vec!["Active".to_owned()]
    } else if states.iter().any(|state| state == "InUse") {
        vec!["InUse".to_owned()]
    } else {
        states
            .first()
            .map(|state| vec![state.clone()])
            .unwrap_or_default()
    }
}

fn contains_state_words(text: &str) -> bool {
    ["state", "phase", "lifecycle", "closed", "active"]
        .iter()
        .any(|needle| text.contains(needle))
}

fn find_type_path(knowledge: &Knowledge, type_id: &TypeId) -> String {
    knowledge
        .types
        .iter()
        .find(|ty| &ty.type_id == type_id)
        .map(|ty| ty.canonical_path.clone())
        .unwrap_or_default()
}

fn is_activate_like(name: &str) -> bool {
    matches!(name, "start" | "open" | "connect" | "begin")
}

fn is_configure_like(name: &str) -> bool {
    matches!(name, "configure" | "set_config" | "configure_with")
}

fn is_close_like(name: &str) -> bool {
    matches!(name, "close" | "finish" | "shutdown" | "end")
}

fn is_recovery_like(name: &str) -> bool {
    matches!(name, "reset" | "retry" | "reconnect" | "recover")
}

fn infer_forbidden_from_state(reason: &str) -> String {
    let lower = reason.to_ascii_lowercase();
    if lower.contains("closed") {
        "Closed".to_owned()
    } else if lower.contains("active") {
        "Active".to_owned()
    } else if lower.contains("construct") || lower.contains("initial") {
        "Constructed".to_owned()
    } else {
        "Unknown".to_owned()
    }
}

fn is_constructor_like(api: &ApiInfo) -> bool {
    matches!(api.api_kind, ApiKind::Constructor | ApiKind::AssocFunction)
}

fn is_mut_receiver(receiver: Option<&str>) -> bool {
    let Some(receiver) = receiver else {
        return false;
    };
    let normalized = receiver
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<String>()
        .to_ascii_lowercase();

    normalized.contains("mutself")
}

fn split_non_empty_lines(section: &str) -> Vec<String> {
    section
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

use seraph_types::{
    ApiId, Knowledge, TraitImplInfo, TraitOrigin, TraitSurfaceEntry, TraitSurfaceKind,
    TraitSurfaceMap, TraitSurfaceSignificance, TypeId, TypeInfo, TypeTraitSurface,
};
use std::collections::{BTreeMap, BTreeSet};

pub fn build_trait_surface_map(knowledge: &Knowledge) -> TraitSurfaceMap {
    let public_types = knowledge
        .types
        .iter()
        .map(|item| (item.type_id.clone(), item))
        .collect::<BTreeMap<TypeId, &TypeInfo>>();
    let trait_method_index = build_trait_method_index(knowledge);
    let mut grouped = BTreeMap::<TypeId, Vec<TraitSurfaceEntry>>::new();

    for impl_info in &knowledge.trait_impl_registry {
        let Some(type_info) = public_types.get(&impl_info.target_type_id) else {
            continue;
        };
        let Some((surface_kind, significance)) = classify_trait_surface(impl_info) else {
            continue;
        };

        grouped
            .entry(type_info.type_id.clone())
            .or_default()
            .push(TraitSurfaceEntry {
                trait_impl_id: impl_info.trait_impl_id.clone(),
                trait_id: impl_info.trait_id.clone(),
                trait_name: impl_info.trait_name.clone(),
                trait_path: impl_info.trait_canonical_path.clone(),
                trait_origin: impl_info.trait_origin.clone(),
                for_type_text: impl_info.for_type_text.clone(),
                surface_kind,
                significance,
                capability_summary: build_capability_summary(impl_info, surface_kind),
                trait_method_api_ids: trait_method_index
                    .get(&impl_info.trait_id)
                    .cloned()
                    .unwrap_or_default(),
                associated_type_bindings: impl_info.associated_type_bindings.clone(),
                associated_const_bindings: impl_info.associated_const_bindings.clone(),
                cfg_attrs: impl_info.cfg_attrs.clone(),
                is_unsafe: impl_info.is_unsafe,
            });
    }

    let mut type_surfaces = grouped
        .into_iter()
        .filter_map(|(type_id, mut trait_surfaces)| {
            let type_info = public_types.get(&type_id)?;
            trait_surfaces.sort_by(|left, right| {
                left.trait_path
                    .cmp(&right.trait_path)
                    .then(left.trait_impl_id.cmp(&right.trait_impl_id))
            });
            Some(TypeTraitSurface {
                type_id,
                path: type_info.canonical_path.clone(),
                trait_surfaces,
            })
        })
        .collect::<Vec<_>>();
    type_surfaces.sort_by(|left, right| left.path.cmp(&right.path));

    TraitSurfaceMap { type_surfaces }
}

fn build_trait_method_index(knowledge: &Knowledge) -> BTreeMap<seraph_types::TraitId, Vec<ApiId>> {
    let mut index = BTreeMap::<seraph_types::TraitId, BTreeSet<ApiId>>::new();

    // Trait method API rows live in the Phase 1 API table. We keep only the
    // stable api ids here so Phase 2 can reference the already-extracted nodes.
    for api in &knowledge.apis {
        let Some(owner_trait_id) = &api.owner_trait_id else {
            continue;
        };
        index
            .entry(owner_trait_id.clone())
            .or_default()
            .insert(api.api_id.clone());
    }

    index
        .into_iter()
        .map(|(trait_id, api_ids)| (trait_id, api_ids.into_iter().collect()))
        .collect()
}

fn classify_trait_surface(
    impl_info: &TraitImplInfo,
) -> Option<(TraitSurfaceKind, TraitSurfaceSignificance)> {
    let name = impl_info.trait_name.as_str();
    let path = impl_info.trait_canonical_path.as_str();

    if matches!(impl_info.trait_origin, TraitOrigin::Local) {
        return Some((
            TraitSurfaceKind::DomainTrait,
            TraitSurfaceSignificance::High,
        ));
    }
    if is_low_value_trait(name) {
        return None;
    }
    if is_iteration_trait(name) {
        return Some((TraitSurfaceKind::Iteration, TraitSurfaceSignificance::High));
    }
    if is_high_signal_io_trait(name, path) {
        return Some((TraitSurfaceKind::IoAdapter, TraitSurfaceSignificance::High));
    }
    if is_construction_trait(name) {
        return Some((
            TraitSurfaceKind::Construction,
            TraitSurfaceSignificance::Medium,
        ));
    }
    if is_adapter_trait(name) {
        return Some((TraitSurfaceKind::Adapter, TraitSurfaceSignificance::Medium));
    }
    if is_mutation_extension_trait(name) {
        return Some((
            TraitSurfaceKind::MutationExtension,
            TraitSurfaceSignificance::Medium,
        ));
    }
    if is_serialization_trait(name) {
        return Some((
            TraitSurfaceKind::Serialization,
            TraitSurfaceSignificance::Medium,
        ));
    }
    if is_operator_trait(name) {
        return Some((TraitSurfaceKind::Operator, TraitSurfaceSignificance::Medium));
    }
    if is_medium_signal_io_trait(name, path) {
        return Some((TraitSurfaceKind::IoAdapter, TraitSurfaceSignificance::Medium));
    }

    None
}

fn build_capability_summary(impl_info: &TraitImplInfo, surface_kind: TraitSurfaceKind) -> String {
    match surface_kind {
        TraitSurfaceKind::DomainTrait => {
            format!("supports the crate-local {} trait surface", impl_info.trait_name)
        }
        TraitSurfaceKind::Iteration => "can be used through iterator-style consumption".to_owned(),
        TraitSurfaceKind::Construction => format!(
            "supports additional construction via {}",
            impl_info.trait_name
        ),
        TraitSurfaceKind::Adapter => {
            format!("supports adapter-style access via {}", impl_info.trait_name)
        }
        TraitSurfaceKind::IoAdapter => {
            format!("supports I/O-style usage via {}", impl_info.trait_name)
        }
        TraitSurfaceKind::MutationExtension => format!(
            "supports incremental mutation via {}",
            impl_info.trait_name
        ),
        TraitSurfaceKind::Serialization => format!(
            "supports serialization ecosystem access via {}",
            impl_info.trait_name
        ),
        TraitSurfaceKind::Operator => format!(
            "supports operator-style composition via {}",
            impl_info.trait_name
        ),
    }
}

fn is_low_value_trait(name: &str) -> bool {
    matches!(
        name,
        "Clone"
            | "Copy"
            | "Debug"
            | "Display"
            | "LowerHex"
            | "UpperHex"
            | "Binary"
            | "Octal"
            | "Pointer"
            | "Eq"
            | "PartialEq"
            | "Ord"
            | "PartialOrd"
            | "Hash"
            | "Send"
            | "Sync"
            | "Unpin"
            | "RefUnwindSafe"
            | "UnwindSafe"
    )
}

fn is_iteration_trait(name: &str) -> bool {
    matches!(name, "IntoIterator" | "Iterator")
}

fn is_high_signal_io_trait(name: &str, path: &str) -> bool {
    matches!(name, "Read" | "Write" | "BufRead" | "Seek") && !path.contains("fmt::Write")
}

fn is_medium_signal_io_trait(name: &str, path: &str) -> bool {
    name == "Write" && path.contains("fmt::Write")
}

fn is_construction_trait(name: &str) -> bool {
    matches!(name, "Default" | "From" | "TryFrom" | "FromIterator")
}

fn is_adapter_trait(name: &str) -> bool {
    matches!(
        name,
        "Deref" | "DerefMut" | "AsRef" | "AsMut" | "Borrow" | "BorrowMut"
    )
}

fn is_mutation_extension_trait(name: &str) -> bool {
    name == "Extend"
}

fn is_serialization_trait(name: &str) -> bool {
    matches!(name, "Serialize" | "Deserialize")
}

fn is_operator_trait(name: &str) -> bool {
    matches!(
        name,
        "Add"
            | "Sub"
            | "Mul"
            | "Div"
            | "Rem"
            | "BitAnd"
            | "BitOr"
            | "BitXor"
            | "Shl"
            | "Shr"
            | "Neg"
            | "Not"
    )
}

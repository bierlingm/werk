mod fixtures;

use fixtures::small_tree::fixed_now;
use werk_sigil::{
    Ctx, Logic, Scope, ScopeKind, SheetLogic, CompositeLogic, CompositionRule, SigilError,
    load_preset,
};
use werk_core::store::Store;

fn preset_logic(name: &str) -> Logic {
    load_preset(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("presets/{name}.toml")),
    )
    .unwrap()
    .logic
}

fn union_scope(ids: Vec<String>) -> Scope {
    let members = ids
        .into_iter()
        .map(|id| Scope {
            kind: ScopeKind::Tension,
            root: Some(id),
            depth: None,
            name: None,
            status: None,
            members: Vec::new(),
            at: None,
        })
        .collect();
    Scope {
        kind: ScopeKind::Union,
        root: None,
        depth: None,
        name: None,
        status: None,
        members,
        at: None,
    }
}

#[test]
fn tiles_four_sub_sigils() {
    let store = Store::new_in_memory().unwrap();
    let ids = (0..4)
        .map(|idx| {
            let name = format!("root {idx}");
            store.create_tension(&name, "actual").unwrap().id
        })
        .collect::<Vec<_>>();
    let scope = union_scope(ids);
    let logic = preset_logic("glance");
    let mut ctx = Ctx::new(fixed_now(), &store, "werk", 0);
    let sheet = SheetLogic { inner_logic: logic };
    let sigil = sheet.render(scope, &mut ctx).unwrap();
    let svg = String::from_utf8(sigil.svg.0).unwrap();
    assert!(svg.matches("data-sigil-index").count() == 4);
}

#[test]
fn recursion_limit_at_four() {
    let store = Store::new_in_memory().unwrap();
    let scope = union_scope(vec![
        store.create_tension("root", "actual").unwrap().id,
    ]);
    let logic = preset_logic("glance");
    let mut ctx = Ctx::new(fixed_now(), &store, "werk", 0);
    let sheet = SheetLogic { inner_logic: logic };
    let err = sheet.render_with_depth(scope, &mut ctx, 5).unwrap_err();
    assert!(matches!(err, SigilError::RecursionLimit { depth: 5 }));
}

#[test]
fn rejects_non_union_scope() {
    let store = Store::new_in_memory().unwrap();
    let root = store.create_tension("root", "actual").unwrap();
    let scope = Scope {
        kind: ScopeKind::Subtree,
        root: Some(root.id),
        depth: Some(2),
        name: None,
        status: None,
        members: Vec::new(),
        at: None,
    };
    let logic = preset_logic("glance");
    let mut ctx = Ctx::new(fixed_now(), &store, "werk", 0);
    let sheet = SheetLogic { inner_logic: logic };
    let err = sheet.render(scope, &mut ctx).unwrap_err();
    assert!(matches!(err, SigilError::Construction { .. }));
}

#[test]
fn concentric_stacks_rings() {
    let store = Store::new_in_memory().unwrap();
    let a = store.create_tension("outer", "actual").unwrap();
    let b = store.create_tension("inner", "actual").unwrap();
    let logic = preset_logic("glance");
    let mut ctx = Ctx::new(fixed_now(), &store, "werk", 0);
    let composite = CompositeLogic {
        rule: CompositionRule::Concentric,
        pairs: vec![
            (
                Scope {
                    kind: ScopeKind::Tension,
                    root: Some(a.id),
                    depth: None,
                    name: None,
                    status: None,
                    members: Vec::new(),
                    at: None,
                },
                logic.clone(),
            ),
            (
                Scope {
                    kind: ScopeKind::Tension,
                    root: Some(b.id),
                    depth: None,
                    name: None,
                    status: None,
                    members: Vec::new(),
                    at: None,
                },
                logic,
            ),
        ],
    };
    let sigil = composite.render(&mut ctx).unwrap();
    let svg = String::from_utf8(sigil.svg.0).unwrap();
    let outer = svg.find("data-sigil-index=\"0\"").unwrap();
    let inner = svg.find("data-sigil-index=\"1\"").unwrap();
    assert!(outer < inner);
}

#[test]
fn unsupported_rules() {
    let store = Store::new_in_memory().unwrap();
    let root = store.create_tension("root", "actual").unwrap();
    let logic = preset_logic("glance");
    let mut ctx = Ctx::new(fixed_now(), &store, "werk", 0);
    let composite = CompositeLogic {
        rule: CompositionRule::Overlay,
        pairs: vec![(
            Scope {
                kind: ScopeKind::Tension,
                root: Some(root.id),
                depth: None,
                name: None,
                status: None,
                members: Vec::new(),
                at: None,
            },
            logic,
        )],
    };
    let err = composite.render(&mut ctx).unwrap_err();
    assert!(matches!(err, SigilError::Unsupported { .. }));
}

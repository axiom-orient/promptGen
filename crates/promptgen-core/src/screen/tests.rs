use super::*;
const INPUT: &str = include_str!("../../../../examples/screens/styled-populated.json");
#[test]
fn valid_modes_and_unicode() {
    for input in [
        INPUT,
        include_str!("../../../../examples/screens/placeholder.json"),
        include_str!("../../../../examples/screens/styled-empty.json"),
        include_str!("../../../../examples/screens/wireframe-populated.json"),
        include_str!("../../../../examples/screens/loading.json"),
        include_str!("../../../../examples/screens/error.json"),
        include_str!("../../../../examples/screens/mobile.json"),
    ] {
        assert!(compile(input).is_ok());
    }
    let input = INPUT.replace("Orders", "주문; 내역\\n전체");
    let compiled = compile(&input).unwrap();
    assert!(compiled.to_compact_string().contains("주문;"));
}
#[test]
fn rejects_invalid_contracts() {
    for (from, to) in [
        ("\"styled\"", "\"realistic\""),
        ("\"provided_synthetic\"", "\"placeholder\""),
        ("\"populated\"", "\"empty\""),
        ("\"populated\"", "\"invalid\""),
        ("\"region\": \"main\"", "\"region\": \"missing\""),
        ("\"id\": \"title\"", "\"id\": \"brand\""),
        ("\"width\": 1056", "\"width\": 4096"),
        (
            "\"componentState\": \"default\"",
            "\"componentState\": \"broken\"",
        ),
    ] {
        let bad = INPUT.replace(from, to);
        assert_ne!(bad, INPUT, "mutation {from}");
        assert!(compile(&bad).is_err(), "accepted {from} -> {to}");
    }
}

#[test]
fn raster_anchors_share_edges_and_do_not_reuse_css_widths() {
    let r = ScreenRequest::from_json(INPUT).unwrap();
    let side = render::raster_bounds(r.regions[0].bounds, &r);
    let main = render::raster_bounds(r.regions[1].bounds, &r);
    assert_eq!((side.x, side.width, side.height), (0, 269, 1024));
    assert_eq!(side.x + side.width, main.x);
    assert_eq!(main.x + main.width, 1536);
    let prompt = r.render().unwrap();
    assert!(prompt.contains("left=0 top=0 right=269 bottom=1024"));
    assert!(prompt.contains("aspect ratios differ"));
}

#[test]
fn brief_requires_real_enabled_primary_action() {
    let mut r = ScreenRequest::from_json(INPUT).unwrap();
    r.brief = Some(ServiceBrief {
        audience: "Order operators".into(),
        user_task: "Create an order".into(),
        primary_action: "create".into(),
        density: "compact".into(),
    });
    r.validate().unwrap();
    for invalid in ["missing", "title"] {
        r.brief.as_mut().unwrap().primary_action = invalid.into();
        assert!(r.validate().is_err());
    }
    r.brief.as_mut().unwrap().primary_action = "create".into();
    r.elements
        .iter_mut()
        .find(|e| e.id == "create")
        .unwrap()
        .state = "disabled".into();
    assert!(r.validate().is_err());
}

#[test]
fn element_bounds_reject_overflow_and_wrong_coordinate_space() {
    let mut r = ScreenRequest::from_json(INPUT).unwrap();
    let e = r.elements.iter_mut().find(|e| e.id == "create").unwrap();
    e.bounds = Some(Bounds {
        x: 1100,
        y: 20,
        width: 150,
        height: 40,
    });
    r.validate().unwrap();
    for b in [
        Bounds {
            x: 0,
            y: 20,
            width: 100,
            height: 40,
        },
        Bounds {
            x: u64::MAX,
            y: 20,
            width: 100,
            height: 40,
        },
        Bounds {
            x: 1100,
            y: 20,
            width: 0,
            height: 40,
        },
    ] {
        r.elements
            .iter_mut()
            .find(|e| e.id == "create")
            .unwrap()
            .bounds = Some(b);
        assert!(r.validate().is_err());
    }
}

#[test]
fn service_fixtures_are_compilable_and_content_bound() {
    for input in [
        include_str!("../../../../examples/screens/no-results.json"),
        include_str!("../../../../examples/screens/sign-in.json"),
        include_str!("../../../../examples/screens/form-error-ko.json"),
        include_str!("../../../../examples/screens/settings.json"),
        include_str!("../../../../examples/screens/catalog.json"),
        include_str!("../../../../examples/screens/mixed-state.json"),
    ] {
        let compiled = compile(input).unwrap();
        assert_eq!(
            compiled
                .as_object()
                .unwrap()
                .get("inputJson")
                .unwrap()
                .as_str(),
            Some(input)
        );
        let prompt = compiled
            .as_object()
            .unwrap()
            .get("prompt")
            .unwrap()
            .as_str()
            .unwrap();
        assert_eq!(
            compiled
                .as_object()
                .unwrap()
                .get("promptSha256")
                .unwrap()
                .as_str(),
            Some(sha256(prompt).as_str())
        );
        assert_eq!(
            compiled.to_compact_string(),
            compile(input).unwrap().to_compact_string()
        );
    }
}

#[test]
fn placeholder_wireframe_and_region_states_remain_independent() {
    let placeholder = ScreenRequest::from_json(include_str!(
        "../../../../examples/screens/placeholder.json"
    ))
    .unwrap();
    assert!(placeholder.render().unwrap().contains("no readable text"));
    assert!(!placeholder.render().unwrap().contains("Theme tokens"));
    let mut wire = ScreenRequest::from_json(INPUT).unwrap();
    wire.fidelity = Fidelity::Wireframe;
    let prompt = wire.render().unwrap();
    assert!(!prompt.contains("#2563EB"));
    assert!(prompt.contains("ORD-1042"));
    let mixed = ScreenRequest::from_json(include_str!(
        "../../../../examples/screens/mixed-state.json"
    ))
    .unwrap()
    .render()
    .unwrap();
    assert!(mixed.contains("Populated: draw exactly"));
    assert!(mixed.contains("Region-scoped error"));
}

#[test]
fn optional_contract_fields_are_strictly_decoded() {
    let input = include_str!("../../../../examples/screens/sign-in.json");
    for (from, to) in [
        ("\"density\": \"comfortable\"", "\"density\": \"random\""),
        ("\"userTask\":", "\"unknownTask\":"),
        (
            "\"primaryAction\": \"submit\"",
            "\"primaryAction\": \"missing\"",
        ),
    ] {
        let bad = input.replace(from, to);
        assert_ne!(input, bad);
        assert!(compile(&bad).is_err());
    }
}

#[test]
fn direct_typed_render_fails_closed_for_invalid_rows_and_geometry() {
    let mut request = ScreenRequest::from_json(INPUT).unwrap();
    request.regions[0].rows = vec![vec![
        "1".into(),
        "2".into(),
        "3".into(),
        "4".into(),
        "5".into(),
        "6".into(),
        "7".into(),
        "8".into(),
        "9".into(),
    ]];
    request.regions[0].state = DataState::Populated;
    request.regions[0].message.clear();
    assert!(request.render().is_err());

    let mut request = ScreenRequest::from_json(INPUT).unwrap();
    request.viewport.width = 0;
    assert!(request.render().is_err());
}

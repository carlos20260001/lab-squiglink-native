//! Tests for d3-selection and the DOM implementation.

use d3::dom::{create_document, create_element, Dom};
use d3::selection::{
    create, custom_event, local, mouse, namespace, namespaces, Event,
};

#[test]
fn test_dom_node_tree_and_attributes() {
    let doc = create_document();
    assert_eq!(doc.node_type(), 9);

    let html = doc.document_element().expect("html element");
    assert_eq!(html.tag_name(), "html");

    let div = doc.create_element("div");
    div.set_attribute("id", "main");
    div.set_attribute("class", "container active");

    assert_eq!(div.get_attribute("id"), Some("main".to_string()));
    assert!(div.has_attribute("class"));
    assert!(div.class_list_contains("container"));
    assert!(div.class_list_contains("active"));

    div.class_list_remove("active");
    assert!(!div.class_list_contains("active"));
    div.class_list_add("highlight");
    assert!(div.class_list_contains("highlight"));

    html.append_child(div.clone());
    assert_eq!(div.parent_node().unwrap().tag_name(), "html");
    assert_eq!(html.child_nodes().len(), 2); // body + div
}

#[test]
fn test_dom_selectors() {
    let doc = create_document();
    let body = doc.query_selector("body").expect("body");

    let svg = doc.create_element("svg");
    svg.set_attribute("id", "chart");
    body.append_child(svg.clone());

    let g1 = doc.create_element("g");
    g1.set_attribute("class", "tick");
    svg.append_child(g1.clone());

    let line = doc.create_element("line");
    line.set_attribute("stroke", "black");
    g1.append_child(line.clone());

    let text = doc.create_element("text");
    text.set_text_content("42");
    g1.append_child(text.clone());

    let g2 = doc.create_element("g");
    g2.set_attribute("class", "domain");
    svg.append_child(g2.clone());

    // Test selectors
    assert!(body.query_selector("#chart").is_some());
    assert_eq!(body.query_selector("svg").unwrap().id(), svg.id());
    assert_eq!(svg.query_selector_all(".tick").len(), 1);
    assert_eq!(svg.query_selector_all(".domain").len(), 1);
    assert_eq!(svg.query_selector_all("g").len(), 2);
    assert_eq!(g1.query_selector("line").unwrap().id(), line.id());
    assert_eq!(g1.query_selector("text").unwrap().id(), text.id());
    assert_eq!(g1.query_selector("text").unwrap().text_content(), "42");
}

#[test]
fn test_selection_create_and_chaining() {
    let sel = create("div")
        .attr("id", "box")
        .attr("width", 200)
        .style("color", "red")
        .classed("visible selected", true)
        .text("Hello Antigravity");

    assert_eq!(sel.get_attr("id"), Some("box".to_string()));
    assert_eq!(sel.get_attr("width"), Some("200".to_string()));
    assert_eq!(sel.get_style("color"), "red");
    assert!(sel.get_classed("visible"));
    assert!(sel.get_classed("selected"));
    assert_eq!(sel.get_text(), "Hello Antigravity");
}

#[test]
fn test_selection_data_join_enter_append() {
    let svg = create("svg");
    let data = vec![10, 20, 30, 40, 50];

    // Data join with enter.append
    let mut circles = svg.select_all("circle");
    circles.data(&data);

    let entered = circles
        .enter()
        .append("circle")
        .attr("r", 5)
        .attr_data::<i32, _, _>("cx", |d, _| d * 2);

    assert_eq!(entered.size(), 5);

    // Verify created DOM nodes
    let nodes = entered.nodes();
    assert_eq!(nodes.len(), 5);
    for (i, node) in nodes.iter().enumerate() {
        assert_eq!(node.tag_name(), "circle");
        assert_eq!(node.get_attribute("r"), Some("5".to_string()));
        let expected_cx = ((data[i] * 2)).to_string();
        assert_eq!(node.get_attribute("cx"), Some(expected_cx));
        assert_eq!(node.get_data::<i32>(), Some(data[i]));
    }
}

#[test]
fn test_selection_data_join_update_exit() {
    let svg = create("svg");
    let initial_data = vec!["A", "B", "C", "D"];

    let mut sel = svg.select_all("rect");
    sel.data(&initial_data);
    sel.enter().append("rect");

    assert_eq!(svg.select_all("rect").size(), 4);

    // Now update with fewer items
    let new_data = vec!["A", "B"];
    let mut update = svg.select_all("rect");
    update.data(&new_data);

    assert_eq!(update.size(), 2);
    let exit = update.exit();
    assert_eq!(exit.size(), 2);

    exit.remove();
    assert_eq!(svg.select_all("rect").size(), 2);
}

#[test]
fn test_d3_local() {
    let my_local = local::<String>();

    let parent = create_element("div");
    let child = create_element("span");
    parent.append_child(child.clone());

    // Local set on parent should be inherited by child
    my_local.set(&parent, "inherited_value".to_string());
    assert_eq!(my_local.get(&parent), Some("inherited_value".to_string()));
    assert_eq!(my_local.get(&child), Some("inherited_value".to_string()));

    // Override on child
    my_local.set(&child, "child_value".to_string());
    assert_eq!(my_local.get(&child), Some("child_value".to_string()));
    assert_eq!(my_local.get(&parent), Some("inherited_value".to_string()));

    my_local.remove(&child);
    assert_eq!(my_local.get(&child), Some("inherited_value".to_string()));
}

#[test]
fn test_custom_event_and_mouse() {
    let div = create_element("div");
    div.set_bounding_client_rect(50.0, 100.0, 200.0, 200.0);

    let ev = Event {
        event_type: "mousemove".to_string(),
        client_x: 75.0,
        client_y: 125.0,
        ..Default::default()
    };

    custom_event(ev, || {
        let pt = mouse(&div);
        assert_eq!(pt[0], 25.0); // 75 - 50
        assert_eq!(pt[1], 25.0); // 125 - 100
    });
}

#[test]
fn test_namespaces() {
    let ns = namespaces();
    assert_eq!(ns.get("svg"), Some(&"http://www.w3.org/2000/svg"));
    assert_eq!(ns.get("xhtml"), Some(&"http://www.w3.org/1999/xhtml"));

    let parsed = namespace("svg:circle");
    assert_eq!(parsed.space, Some("http://www.w3.org/2000/svg".to_string()));
    assert_eq!(parsed.local, "circle");

    let plain = namespace("div");
    assert_eq!(plain.space, None);
    assert_eq!(plain.local, "div");
}

#[test]
fn test_transition() {
    let div = create("div").attr("opacity", "1.0").style("color", "red");

    let t = div
        .transition(Some("fade"))
        .duration(300.0)
        .delay(50.0)
        .attr("opacity", "0.2")
        .style("color", "blue");

    assert_eq!(t.size(), 1);
    let node = t.node().unwrap();
    assert_eq!(node.get_attribute("opacity"), Some("0.2".to_string()));
    assert_eq!(node.get_style_property("color"), "blue");

    // Interrupt
    div.interrupt(Some("fade"));
}

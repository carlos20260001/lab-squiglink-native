//! The port of d3-voronoi, checked against d3's own JavaScript.

use std::fs;

use d3::voronoi::{voronoi, Diagram, Point, Voronoi, EPSILON, EPSILON2};
use serde_json::Value;

fn fixture() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/voronoi.json");
    let text = fs::read_to_string(path).expect("the fixture");
    serde_json::from_str(&text).expect("the fixture is JSON")
}

fn bits(value: &Value) -> u64 {
    let text = value.as_str().expect("the bits of a number");
    let mut bits = 0u64;
    for i in 0..16 {
        let digit = u8::from_str_radix(&text[i..i + 1], 16).expect("a hex digit");
        bits = (bits << 4) | digit as u64;
    }
    bits
}

fn same_number(a: f64, b: f64) -> bool {
    if a.is_nan() && b.is_nan() {
        true
    } else {
        a.to_bits() == b.to_bits()
    }
}

fn same_point(a: [f64; 2], b: &Value) -> bool {
    let list = b.as_array().expect("a point");
    list.len() == 2
        && same_number(a[0], f64::from_bits(bits(&list[0])))
        && same_number(a[1], f64::from_bits(bits(&list[1])))
}

#[test]
fn basic_edges_and_cells() {
    let f = fixture();
    let data: Vec<[f64; 2]> = vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0], [0.5, 0.5]];
    let dia: Diagram<[f64; 2]> = voronoi().call(&data);
    let e = &f["basic"];
    // edges count
    let exp_edges = e["edges"].as_array().unwrap();
    assert_eq!(dia.edges.len(), exp_edges.len());
    for (i, ee) in dia.edges.iter().enumerate() {
        let ex = &exp_edges[i];
        if ex.is_null() {
            assert!(ee.is_none());
        } else {
            let arr = ex.as_array().unwrap();
            let got = ee.as_ref().unwrap();
            // endpoints
            for k in 0..2 {
                if arr[k].is_null() {
                    assert!(got.v[k].is_none());
                } else {
                    assert!(same_point(got.v[k].unwrap(), &arr[k]));
                }
            }
        }
    }
    // cells count
    let exp_cells = e["cells"].as_array().unwrap();
    assert_eq!(dia.cells.len(), exp_cells.len());
    for (i, cc) in dia.cells.iter().enumerate() {
        let ex = &exp_cells[i];
        if ex.is_null() {
            assert!(cc.is_none());
        } else {
            let cell = cc.as_ref().unwrap();
            assert!(same_point(cell.site.point, &ex["site"]));
            assert_eq!(
                cell.halfedges,
                ex["halfedges"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_u64().unwrap() as usize)
                    .collect::<Vec<_>>()
            );
        }
    }
}

#[test]
fn basic_polygons_triangles_links() {
    let f = fixture();
    let data: Vec<[f64; 2]> = vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0], [0.5, 0.5]];
    let dia = voronoi::<[f64; 2]>().call(&data);
    let e = &f["basic"];

    // polygons
    let exp_polys = e["polygons"].as_array().unwrap();
    let got_polys = dia.polygons();
    assert_eq!(got_polys.len(), exp_polys.len());
    for (gp, ep) in got_polys.iter().zip(exp_polys.iter()) {
        if ep.is_null() {
            assert!(gp.is_none());
        } else {
            let g = gp.as_ref().unwrap();
            let pts = ep["points"].as_array().unwrap();
            assert_eq!(g.points.len(), pts.len());
            for (a, b) in g.points.iter().zip(pts.iter()) {
                if b.is_null() {
                    assert!(a.is_none());
                } else {
                    assert!(same_point(a.unwrap(), b));
                }
            }
            assert!(same_point(g.data, &ep["data"]));
        }
    }

    // triangles
    let exp_tris = e["triangles"].as_array().unwrap();
    let got_tris = dia.triangles();
    assert_eq!(got_tris.len(), exp_tris.len());
    for (gt, et) in got_tris.iter().zip(exp_tris.iter()) {
        let et = et.as_array().unwrap();
        for k in 0..3 {
            assert!(same_point(gt[k], &et[k]));
        }
    }

    // links
    let exp_links = e["links"].as_array().unwrap();
    let got_links = dia.links();
    assert_eq!(got_links.len(), exp_links.len());
    for (gl, el) in got_links.iter().zip(exp_links.iter()) {
        assert!(same_point(gl.source, &el["source"]));
        assert!(same_point(gl.target, &el["target"]));
    }
}

#[test]
fn find_basic() {
    let f = fixture();
    let data: Vec<[f64; 2]> = vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0], [0.5, 0.5]];
    let mut dia = voronoi::<[f64; 2]>().call(&data);
    let e = &f["basic"];
    if let Some(ptexp) = e["find00"].as_array() {
        let got = dia.find(0.0, 0.0, None).unwrap();
        assert!(same_point(got.point, &serde_json::json!(ptexp)));
    }
    assert!(dia.find(10.0, 10.0, Some(1.0)).is_none());
}

#[test]
fn extent_case() {
    let f = fixture();
    let data: Vec<[f64; 2]> = vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0], [0.5, 0.5]];
    let v = Voronoi::<[f64; 2]>::new().extent([[0.0, 0.0], [2.0, 2.0]]);
    let dia = v.call(&data);
    let e = &f["extent"];
    let got_polys = dia.polygons();
    let exp_polys = e["polygons"].as_array().unwrap();
    assert_eq!(got_polys.len(), exp_polys.len());
    for (gp, ep) in got_polys.iter().zip(exp_polys.iter()) {
        if ep.is_null() {
            assert!(gp.is_none());
            continue;
        }
        let g = gp.as_ref().unwrap();
        let pts = ep.as_array().unwrap();
        assert_eq!(g.points.len(), pts.len());
        for (a, b) in g.points.iter().zip(pts.iter()) {
            if b.is_null() {
                assert!(a.is_none());
            } else {
                assert!(same_point(a.unwrap(), b));
            }
        }
    }
    // triangles
    let got_tris = dia.triangles();
    let exp_tris = e["triangles"].as_array().unwrap();
    assert_eq!(got_tris.len(), exp_tris.len());
    for (gt, et) in got_tris.iter().zip(exp_tris.iter()) {
        let et = et.as_array().unwrap();
        for k in 0..3 {
            assert!(same_point(gt[k], &et[k]));
        }
    }
    // links count
    let got_links = dia.links();
    let exp_links = e["links"].as_array().unwrap();
    assert_eq!(got_links.len(), exp_links.len());
    // find
    let mut dia2 = v.call(&data);
    let got_find = dia2.find(0.1, 0.1, None).unwrap();
    let exp_find = e["findNear"].as_array().unwrap();
    assert!(same_point(
        got_find.point,
        &serde_json::Value::Array(exp_find.clone())
    ));
    assert!(dia2.find(10.0, 10.0, Some(1.0)).is_none());
}

#[test]
fn size_case() {
    let f = fixture();
    let data: Vec<[f64; 2]> = vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0], [0.5, 0.5]];
    let mut v = Voronoi::<[f64; 2]>::new();
    v.set_size([2.0, 2.0]);
    let dia = v.call(&data);
    let e = &f["size"];
    assert_eq!(
        dia.polygons().len(),
        e["polygonsCount"].as_u64().unwrap() as usize
    );
    let _got_size = v.get_size().unwrap();
    let exp_extent = e["extent"].as_array().unwrap(); // not stored? fixture has extent under size
                                                      // extent getter
    let ext = v.get_extent().unwrap();
    assert!(same_number(
        ext[0][0],
        f64::from_bits(bits(&exp_extent[0].as_array().unwrap()[0]))
    ));
}

#[test]
fn constant_case() {
    let f = fixture();
    let data: Vec<[f64; 2]> = vec![[0.0, 0.0], [1.0, 0.0]];
    let v = Voronoi::<[f64; 2]>::new().x(5.0).y(7.0);
    let dia = v.call(&data);
    let e = &f["constant"];
    let got_polys = dia.polygons();
    let exp_polys = e["polygons"].as_array().unwrap();
    assert_eq!(got_polys.len(), exp_polys.len());
    for (gp, ep) in got_polys.iter().zip(exp_polys.iter()) {
        if ep.is_null() {
            assert!(gp.is_none());
            continue;
        }
        let g = gp.as_ref().unwrap();
        let pts = ep.as_array().unwrap();
        assert_eq!(g.points.len(), pts.len());
        for (a, b) in g.points.iter().zip(pts.iter()) {
            if b.is_null() {
                assert!(a.is_none());
            } else {
                assert!(same_point(a.unwrap(), b));
            }
        }
    }
}

#[test]
fn accessor_case() {
    let f = fixture();
    #[derive(Clone)]
    struct P {
        a: f64,
        b: f64,
    }
    impl Point for P {
        fn x(&self) -> f64 {
            self.a
        }
        fn y(&self) -> f64 {
            self.b
        }
    }
    let data = vec![
        P { a: 0.0, b: 0.0 },
        P { a: 2.0, b: 0.0 },
        P { a: 1.0, b: 2.0 },
    ];
    let v = Voronoi::with_accessors(|d: &P| d.a, |d: &P| d.b);
    let dia = v.call(&data);
    let e = &f["accessor"];
    // polygons
    let got_polys = dia.polygons();
    let exp_polys = e["polygons"].as_array().unwrap();
    assert_eq!(got_polys.len(), exp_polys.len());
    for (gp, ep) in got_polys.iter().zip(exp_polys.iter()) {
        let g = gp.as_ref().unwrap();
        let pts = ep["points"].as_array().unwrap();
        assert_eq!(g.points.len(), pts.len());
        for (a, b) in g.points.iter().zip(pts.iter()) {
            if b.is_null() {
                assert!(a.is_none());
            } else {
                assert!(same_point(a.unwrap(), b));
            }
        }
        let dexp = ep["data"].as_array().unwrap();
        assert!(same_number(g.data.a, f64::from_bits(bits(&dexp[0]))));
        assert!(same_number(g.data.b, f64::from_bits(bits(&dexp[1]))));
    }
    // triangles
    let got_tris = dia.triangles();
    let exp_tris = e["triangles"].as_array().unwrap();
    assert_eq!(got_tris.len(), exp_tris.len());
    // links
    let got_links = dia.links();
    let exp_links = e["links"].as_array().unwrap();
    assert_eq!(got_links.len(), exp_links.len());
}

#[test]
fn single_case() {
    let f = fixture();
    let data: Vec<[f64; 2]> = vec![[5.0, 5.0]];
    let v = Voronoi::<[f64; 2]>::new().extent([[0.0, 0.0], [10.0, 10.0]]);
    let dia = v.call(&data);
    let e = &f["single"];
    let got_polys = dia.polygons();
    let exp_polys = e["polygons"].as_array().unwrap();
    assert_eq!(got_polys.len(), exp_polys.len());
    for (gp, ep) in got_polys.iter().zip(exp_polys.iter()) {
        if ep.is_null() {
            assert!(gp.is_none());
            continue;
        }
        let g = gp.as_ref().unwrap();
        let pts = ep.as_array().unwrap();
        assert_eq!(g.points.len(), pts.len());
        for (a, b) in g.points.iter().zip(pts.iter()) {
            if b.is_null() {
                assert!(a.is_none());
            } else {
                assert!(same_point(a.unwrap(), b));
            }
        }
    }
    assert!(dia.triangles().is_empty());
    assert!(dia.links().is_empty());
}

#[test]
fn duplicates_case() {
    let f = fixture();
    let data: Vec<[f64; 2]> = vec![[0.0, 0.0], [0.0, 0.0], [1.0, 1.0]];
    let v = Voronoi::<[f64; 2]>::new().extent([[0.0, 0.0], [2.0, 2.0]]);
    let dia = v.call(&data);
    let e = &f["duplicates"];
    let got_polys = dia.polygons();
    let exp_polys = e["polygons"].as_array().unwrap();
    assert_eq!(got_polys.len(), exp_polys.len());
    for (gp, ep) in got_polys.iter().zip(exp_polys.iter()) {
        if ep.is_null() {
            assert!(gp.is_none());
            continue;
        }
        let g = gp.as_ref().unwrap();
        let pts = ep.as_array().unwrap();
        assert_eq!(g.points.len(), pts.len());
        for (a, b) in g.points.iter().zip(pts.iter()) {
            if b.is_null() {
                assert!(a.is_none());
            } else {
                assert!(same_point(a.unwrap(), b));
            }
        }
    }
    let got_tris = dia.triangles();
    let exp_tris = e["triangles"].as_array().unwrap();
    assert_eq!(got_tris.len(), exp_tris.len());
}

#[test]
fn empty_case() {
    let f = fixture();
    let data: Vec<[f64; 2]> = vec![];
    let dia = voronoi::<[f64; 2]>().call(&data);
    let e = &f["empty"];
    assert_eq!(dia.edges.len(), e["edges"].as_u64().unwrap() as usize);
    assert_eq!(dia.cells.len(), e["cells"].as_u64().unwrap() as usize);
    assert_eq!(
        dia.polygons().len(),
        e["polygons"].as_u64().unwrap() as usize
    );
    assert_eq!(
        dia.triangles().len(),
        e["triangles"].as_u64().unwrap() as usize
    );
    assert_eq!(dia.links().len(), e["links"].as_u64().unwrap() as usize);
}

#[test]
fn getters() {
    let v = Voronoi::<[f64; 2]>::new();
    assert!(v.get_extent().is_none());
    assert!(v.get_size().is_none());
    let mut v2 = v;
    v2.set_extent(Some([[1.0, 2.0], [3.0, 4.0]]));
    assert_eq!(v2.get_extent().unwrap(), [[1.0, 2.0], [3.0, 4.0]]);
    let mut v3 = Voronoi::<[f64; 2]>::new();
    v3.set_size(Some([2.0, 3.0]));
    assert_eq!(v3.get_size().unwrap(), [2.0, 3.0]);
    assert_eq!(v3.get_extent().unwrap(), [[0.0, 0.0], [2.0, 3.0]]);
}

#[test]
fn consuming_setters_chain() {
    let data: Vec<[f64; 2]> = vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]];
    let dia = Voronoi::<[f64; 2]>::new()
        .x(|d: &[f64; 2]| d[0])
        .y(|d: &[f64; 2]| d[1])
        .size([2.0, 2.0])
        .call(&data);
    assert_eq!(dia.cells.len(), 3);
}

#[test]
fn edge_index_access() {
    let data: Vec<[f64; 2]> = vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]];
    let v = Voronoi::<[f64; 2]>::new().extent([[0.0, 0.0], [2.0, 2.0]]);
    let dia = v.call(&data);
    if let Some(e) = &dia.edges[0] {
        let _ = e[0];
        let _ = e[1];
    }
}

#[test]
fn point_trait_vec() {
    let data: Vec<Vec<f64>> = vec![vec![0.0, 0.0], vec![1.0, 0.0], vec![0.0, 1.0]];
    let dia = voronoi::<Vec<f64>>().call(&data);
    assert_eq!(dia.cells.len(), 3);
}

#[test]
fn constants() {
    assert_eq!(EPSILON, 1e-6);
    assert_eq!(EPSILON2, 1e-12);
}

#[test]
fn free_function_voronoi() {
    let data: Vec<[f64; 2]> = vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]];
    let dia = voronoi::<[f64; 2]>().call(&data);
    assert_eq!(dia.cells.len(), 3);
}

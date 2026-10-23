//! The port of d3-contour.

use crate::array::{extent, range, threshold_sturges, tick_step};

#[derive(Clone, Debug)]
pub struct Contour {
    pub r#type: &'static str,
    pub value: f64,
    pub coordinates: Vec<Vec<Vec<[f64; 2]>>>,
}

#[derive(Clone)]
enum Tz {
    Number(f64),
    Array(Vec<f64>),
}

pub struct Contours {
    dx: usize,
    dy: usize,
    threshold: Box<dyn Fn(&[f64]) -> Tz>,
    smooth: bool,
}

impl Contours {
    pub fn new() -> Self {
        Contours {
            dx: 1,
            dy: 1,
            threshold: Box::new(|vals: &[f64]| Tz::Number(threshold_sturges(vals))),
            smooth: true,
        }
    }

    pub fn size(&mut self, s: [usize; 2]) -> &mut Self {
        let x = s[0].max(1);
        let y = s[1].max(1);
        self.dx = x;
        self.dy = y;
        self
    }

    pub fn size_get(&self) -> [usize; 2] { [self.dx, self.dy] }

    pub fn thresholds<F>(&mut self, f: F) -> &mut Self
    where
        F: Fn(&[f64]) -> Tz + 'static,
    {
        self.threshold = Box::new(f);
        self
    }

    // convenience: set thresholds from a number
    pub fn thresholds_number(&mut self, n: f64) -> &mut Self {
        self.threshold = Box::new(move |_vals: &[f64]| Tz::Number(n));
        self
    }

    pub fn thresholds_array(&mut self, arr: Vec<f64>) -> &mut Self {
        let a = arr;
        self.threshold = Box::new(move |_vals: &[f64]| Tz::Array(a.clone()));
        self
    }

    pub fn smooth(&mut self, b: bool) -> &mut Self { self.smooth = b; self }
    pub fn smooth_get(&self) -> bool { self.smooth }

    pub fn call(&self, values: &[f64]) -> Vec<Contour> {
        let tz = (self.threshold)(values);
        let thresholds: Vec<f64> = match tz {
            Tz::Number(n) => {
                let (start, stop) = extent(values);
                let s = start.unwrap_or(0.0);
                let e = stop.unwrap_or(0.0);
                let step = tick_step(s, e, n);
                let first = (s / step).floor() * step;
                let last = (e / step).floor() * step;
                let mut r = range(first, Some(last + step*0.5), Some(step));
                r.retain(|&v| v < e + 1e-9 || (v-e).abs() < 1e-9);
                // remove last if >= stop
                while r.last().map(|&v| v >= e).unwrap_or(false) { r.pop(); }
                r
            }
            Tz::Array(mut a) => { a.sort_by(|x,y| x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal)); a }
        };

        thresholds.into_iter().map(|v| self.contour(values, v)).collect()
    }

    pub fn contour(&self, values: &[f64], value: f64) -> Contour {
        let dx = self.dx;
        let dy = self.dy;
        let mut polygons: Vec<Vec<Vec<[f64;2]>>> = Vec::new();
        let mut holes: Vec<Vec<[f64;2]>> = Vec::new();

        let mut nring = 0;
        self.isorings(values, value, &mut |ring: Vec<[f64;2]>| {
            let mut r = ring;
            if self.smooth { smooth_linear(&mut r, values, value, dx, dy); }
            if area(&r) < 0.0 { polygons.push(vec![r]); }
            else { holes.push(r); }
        });

        for hole in &holes {
            for poly in polygons.iter_mut() {
                if contains(&poly[0], hole) != -1 {
                    poly.push(hole.clone());
                    break;
                }
            }
        }

        Contour { r#type: "MultiPolygon", value, coordinates: polygons }
    }

    fn isorings(&self, values: &[f64], value: f64, callback: &mut dyn FnMut(Vec<[f64;2]>)) {
        let dx = self.dx;
        let dy = self.dy;
        // fragment storage indexed by index()
        let mut frag_start: Vec<Option<usize>> = vec![None; (dx + 1) * (dy + 1) * 8 + 10];
        let mut frag_end: Vec<Option<usize>> = vec![None; (dx + 1) * (dy + 1) * 8 + 10];
        // store fragments in a vec; reference by... use Option<usize> into fragments vec
        // We'll use Vec<Option<Fragment>> sized generously
        let nfrag = (dx + 1) * (dy + 1) * 8 + 10;
        let mut fragments: Vec<Option<Fragment>> = vec![None; nfrag];

        // helper closures
        let idx = |point: [f64;2]| -> usize {
            (point[0] * 2.0 + point[1] * (dx as f64 + 1.0) * 4.0) as usize
        };

        // stitch closure
        let mut stitch = |line: &[[f64;2];2], x: isize, y: isize,
                          fragments: &mut Vec<Option<Fragment>>,
                          frag_start: &mut Vec<Option<usize>>,
                          frag_end: &mut Vec<Option<usize>>,
                          callback: &mut dyn FnMut(Vec<[f64;2]>)| {
            let start = [line[0][0] + x as f64, line[0][1] + y as f64];
            let end = [line[1][0] + x as f64, line[1][1] + y as f64];
            let start_index = idx(start);
            let end_index = idx(end);

            // f = fragmentByEnd[startIndex]
            if let Some(fi) = frag_end[start_index] {
                let f = fi;
                // g = fragmentByStart[endIndex]
                if let Some(gi) = frag_start[end_index] {
                    let g = gi;
                    // delete
                    if let Some(fr) = &fragments[f] { let e = fr.end; frag_end[e] = None; }
                    if let Some(fr) = &fragments[g] { let s = fr.start; frag_start[s] = None; }
                    if f == g {
                        // close ring
                        if let Some(fr) = &mut fragments[f] {
                            fr.ring.push(end);
                            let ring = fr.ring.clone();
                            fragments[f] = None;
                            callback(ring);
                        }
                    } else {
                        // merge
                        if let (Some(fr_f), Some(fr_g)) = (&fragments[f], &fragments[g]) {
                            let new_ring = {
                                let mut r = fr_f.ring.clone();
                                r.extend(fr_g.ring.clone());
                                r
                            };
                            let new_start = {
                                if let Some(fr) = &fragments[f] { fr.start } else { 0 }
                            };
                            let new_end = {
                                if let Some(fr) = &fragments[g] { fr.end } else { 0 }
                            };
                            // remove old
                            fragments[f] = None;
                            fragments[g] = None;
                            // store merged at f slot? Use a new slot f
                            fragments[f] = Some(Fragment { start: new_start, end: new_end, ring: new_ring });
                            frag_start[new_start] = Some(f);
                            frag_end[new_end] = Some(f);
                        }
                    }
                } else {
                    // f exists, no g
                    if let Some(fr) = &mut fragments[f] {
                        // delete fragmentByEnd[f.end]
                        let old_end = fr.end;
                        frag_end[old_end] = None;
                        fr.ring.push(end);
                        fr.end = end_index;
                        frag_end[end_index] = Some(f);
                    }
                }
            } else if let Some(fi) = frag_start[end_index] {
                let f = fi;
                if let Some(gi) = frag_end[start_index] {
                    let g = gi;
                    if let Some(fr) = &fragments[f] { let s = fr.start; frag_start[s] = None; }
                    if let Some(fr) = &fragments[g] { let e = fr.end; frag_end[e] = None; }
                    if f == g {
                        if let Some(fr) = &mut fragments[f] {
                            fr.ring.push(end);
                            let ring = fr.ring.clone();
                            fragments[f] = None;
                            callback(ring);
                        }
                    } else {
                        if let (Some(fr_f), Some(fr_g)) = (&fragments[f], &fragments[g]) {
                            let new_ring = {
                                let mut r = fr_g.ring.clone();
                                r.extend(fr_f.ring.clone());
                                r
                            };
                            let new_start = fr_g.start;
                            let new_end = fr_f.end;
                            fragments[f] = None;
                            fragments[g] = None;
                            fragments[g] = Some(Fragment { start: new_start, end: new_end, ring: new_ring });
                            frag_start[new_start] = Some(g);
                            frag_end[new_end] = Some(g);
                        }
                    }
                } else {
                    if let Some(fr) = &mut fragments[f] {
                        let old_start = fr.start;
                        frag_start[old_start] = None;
                        fr.ring.insert(0, start);
                        fr.start = start_index;
                        frag_start[start_index] = Some(f);
                    }
                }
            } else {
                // new fragment
                // find free slot
                let mut slot = None;
                for i in 0..fragments.len() {
                    if fragments[i].is_none() { slot = Some(i); break; }
                }
                let slot = slot.unwrap_or(0);
                fragments[slot] = Some(Fragment { start: start_index, end: end_index, ring: vec![start, end] });
                frag_start[start_index] = Some(slot);
                frag_end[end_index] = Some(slot);
            }
        };

        // Special case first row (y = -1, t2 = t3 = 0)
        let mut x: isize = -1;
        let mut y: isize = -1;
        let mut t1 = values[0] >= value;
        for line in &cases_for((t1 as usize) << 1) {
            stitch(line, x, y, &mut fragments, &mut frag_start, &mut frag_end, callback);
        }
        while (x + 1) < (dx as isize - 1) {
            x += 1;
            let t0 = t1;
            t1 = values[(x as usize + 1)] >= value;
            for line in &cases_for((t0 as usize) | ((t1 as usize) << 1)) {
                stitch(line, x, y, &mut fragments, &mut frag_start, &mut frag_end, callback);
            }
        }
        x = dx as isize - 1;
        // last in first row: t1<<0
        {
            let t1v = values[dx-1] >= value; // not exactly; use t1 from loop end
            let _ = t1v;
        }
        for line in &cases_for((t1 as usize) << 0) {
            stitch(line, x, y, &mut fragments, &mut frag_start, &mut frag_end, callback);
        }

        // General intermediate rows
        while (y + 1) < (dy as isize - 1) {
            y += 1;
            x = -1;
            let mut t1 = values[y as usize * dx + dx] >= value;
            let mut t2 = values[y as usize * dx] >= value;
            for line in &cases_for(((t1 as usize) << 1) | ((t2 as usize) << 2)) {
                stitch(line, x, y, &mut fragments, &mut frag_start, &mut frag_end, callback);
            }
            while (x + 1) < (dx as isize - 1) {
                x += 1;
                let t0 = t1;
                t1 = values[y as usize * dx + dx + x as usize + 1] >= value;
                let t3 = t2;
                t2 = values[y as usize * dx + x as usize + 1] >= value;
                for line in &cases_for((t0 as usize) | ((t1 as usize) << 1) | ((t2 as usize) << 2) | ((t3 as usize) << 3)) {
                    stitch(line, x, y, &mut fragments, &mut frag_start, &mut frag_end, callback);
                }
            }
            x = dx as isize - 1;
            for line in &cases_for((t1 as usize) | ((t2 as usize) << 3)) {
                stitch(line, x, y, &mut fragments, &mut frag_start, &mut frag_end, callback);
            }
        }

        // Last row
        y = dy as isize - 1;
        x = -1;
        let mut t2 = values[y as usize * dx] >= value;
        for line in &cases_for((t2 as usize) << 2) {
            stitch(line, x, y, &mut fragments, &mut frag_start, &mut frag_end, callback);
        }
        while (x + 1) < (dx as isize - 1) {
            x += 1;
            let t3 = t2;
            t2 = values[y as usize * dx + x as usize + 1] >= value;
            for line in &cases_for(((t2 as usize) << 2) | ((t3 as usize) << 3)) {
                stitch(line, x, y, &mut fragments, &mut frag_start, &mut frag_end, callback);
            }
        }
        x = dx as isize - 1;
        for line in &cases_for((t2 as usize) << 3) {
            stitch(line, x, y, &mut fragments, &mut frag_start, &mut frag_end, callback);
        }
    }
}

#[derive(Clone)]
struct Fragment {
    start: usize,
    end: usize,
    ring: Vec<[f64;2]>,
}

fn cases_for(i: usize) -> Vec<[[f64; 2]; 2]> {
    match i & 15 {
        0 => vec![],
        1 => vec![[[1.0,1.5],[0.5,1.0]]],
        2 => vec![[[1.5,1.0],[1.0,1.5]]],
        3 => vec![[[1.5,1.0],[0.5,1.0]]],
        4 => vec![[[1.0,0.5],[1.5,1.0]]],
        5 => vec![[[1.0,1.5],[0.5,1.0]],[[1.0,0.5],[1.5,1.0]]],
        6 => vec![[[1.0,0.5],[1.0,1.5]]],
        7 => vec![[[1.0,0.5],[0.5,1.0]]],
        8 => vec![[[0.5,1.0],[1.0,0.5]]],
        9 => vec![[[1.0,1.5],[1.0,0.5]]],
        10 => vec![[[0.5,1.0],[1.0,0.5]],[[1.5,1.0],[1.0,1.5]]],
        11 => vec![[[1.5,1.0],[1.0,0.5]]],
        12 => vec![[[0.5,1.0],[1.5,1.0]]],
        13 => vec![[[1.0,1.5],[1.5,1.0]]],
        14 => vec![[[0.5,1.0],[1.0,1.5]]],
        _ => vec![],
    }
}

fn area(ring: &[[f64;2]]) -> f64 {
    let mut s = 0.0;
    for i in 0..ring.len() {
        let a = ring[i];
        let b = ring[(i+1)%ring.len()];
        s += a[0]*b[1] - a[1]*b[0];
    }
    s / 2.0
}

fn contains(ring: &[[f64;2]], hole: &[[f64;2]]) -> i32 {
    // point in polygon winding for first point of hole
    let p = hole[0];
    let mut winding = 0;
    for i in 0..ring.len() {
        let a = ring[i];
        let b = ring[(i+1)%ring.len()];
        if a[1] <= p[1] {
            if b[1] > p[1] && (b[0]-a[0])*(p[1]-a[1]) > (b[1]-a[1])*(p[0]-a[0]) {
                winding += 1;
            }
        } else {
            if b[1] <= p[1] && (b[0]-a[0])*(p[1]-a[1]) < (b[1]-a[1])*(p[0]-a[0]) {
                winding -= 1;
            }
        }
    }
    if winding != 0 { 1 } else { -1 }
}

fn smooth_linear(ring: &mut Vec<[f64;2]>, values: &[f64], value: f64, dx: usize, dy: usize) {
    for point in ring.iter_mut() {
        let x = point[0];
        let y = point[1];
        let xt = x as usize; // floor for positive
        let yt = y as usize;
        let mut v1 = if yt < dy && xt < dx { values[yt*dx + xt] } else { 0.0 };
        if x > 0.0 && x < dx as f64 && (xt as f64) == x {
            let v0 = if yt < dy && xt > 0 { values[yt*dx + xt - 1] } else { 0.0 };
            point[0] = x + (value - v0)/(v1 - v0) - 0.5;
        }
        if y > 0.0 && y < dy as f64 && (yt as f64) == y {
            let v0 = if yt > 0 && xt < dx { values[(yt-1)*dx + xt] } else { 0.0 };
            // v1 recompute
            v1 = if yt < dy && xt < dx { values[yt*dx + xt] } else { 0.0 };
            point[1] = y + (value - v0)/(v1 - v0) - 0.5;
        }
    }
}

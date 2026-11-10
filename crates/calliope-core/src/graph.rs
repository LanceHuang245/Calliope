//! The knowledge graph: notes as nodes, links as edges, from the index, and
//! where a force-directed layout puts them.

use std::collections::{BTreeSet, HashMap};

use crate::{index::Index, note};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    /// The note's path, or what the links to a note not yet created name.
    pub id: String,
    pub label: String,
    /// False for a note not yet created.
    pub exists: bool,
    /// How many edges it has.
    pub degree: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Graph {
    pub nodes: Vec<Node>,
    /// The linked pairs, as indexes of `nodes`.
    pub edges: Vec<(usize, usize)>,
}

impl Graph {
    /// The graph of the index's notes; `orphans` keeps the notes without
    /// links, `missing` the notes linked to but not yet created.
    pub fn of(index: &Index, orphans: bool, missing: bool) -> Self {
        let mut nodes: Vec<Node> = index.notes().map(|path| node(path, true)).collect();
        let mut ids: HashMap<String, usize> =
            (nodes.iter().map(|n| n.id.clone())).zip(0..).collect();
        let mut edges = BTreeSet::new();
        for from in 0..ids.len() {
            for link in index.links(&nodes[from].id.clone()) {
                let to = match index.resolve(&link.target) {
                    Some(path) => ids[path],
                    None if !missing => continue,
                    None => *ids.entry(link.target.clone()).or_insert_with(|| {
                        nodes.push(node(&link.target, false));
                        nodes.len() - 1
                    }),
                };
                if from != to {
                    edges.insert((from.min(to), from.max(to)));
                }
            }
        }
        for &(a, b) in &edges {
            nodes[a].degree += 1;
            nodes[b].degree += 1;
        }
        // Without the orphans, the nodes left are numbered again.
        let mut kept = vec![usize::MAX; nodes.len()];
        let mut graph = Graph::default();
        for (i, node) in nodes.into_iter().enumerate() {
            if orphans || node.degree > 0 {
                kept[i] = graph.nodes.len();
                graph.nodes.push(node);
            }
        }
        graph.edges = edges.into_iter().map(|(a, b)| (kept[a], kept[b])).collect();
        graph
    }

    /// A position for each node, around the origin: linked nodes pulled
    /// together, all nodes pushed apart (Fruchterman–Reingold, the
    /// repulsion approximated with a Barnes–Hut quadtree).
    pub fn layout(&self) -> Vec<(f32, f32)> {
        let n = self.nodes.len();
        // A spiral to start from: the same layout every time.
        let mut at: Vec<(f32, f32)> = (0..n)
            .map(|i| {
                let (radius, angle) = (SPACING * (i as f32).sqrt(), i as f32 * 2.4);
                (radius * angle.cos(), radius * angle.sin())
            })
            .collect();
        // Fewer rounds for a large graph: each one costs more.
        let rounds = (150_000 / n.max(1)).clamp(60, 300);
        for round in 0..rounds {
            let tree = Tree::new(&at);
            let mut push: Vec<(f32, f32)> = at
                .iter()
                .map(|&(x, y)| {
                    let (fx, fy) = tree.repulsion(0, x, y);
                    // A light pull to the middle keeps separate groups near.
                    (fx - x * GRAVITY, fy - y * GRAVITY)
                })
                .collect();
            for &(a, b) in &self.edges {
                let (dx, dy) = (at[b].0 - at[a].0, at[b].1 - at[a].1);
                let pull = (dx * dx + dy * dy).sqrt() / SPACING;
                push[a].0 += dx * pull;
                push[a].1 += dy * pull;
                push[b].0 -= dx * pull;
                push[b].1 -= dy * pull;
            }
            // Each round moves the nodes less far.
            let step = SPACING * 2.0 * (1.0 - round as f32 / rounds as f32);
            for (p, (fx, fy)) in at.iter_mut().zip(push) {
                let force = (fx * fx + fy * fy).sqrt().max(f32::EPSILON);
                let moved = force.min(step) / force;
                *p = (p.0 + fx * moved, p.1 + fy * moved);
            }
        }
        at
    }
}

fn node(id: &str, exists: bool) -> Node {
    Node {
        id: id.to_owned(),
        label: note::title(id).to_owned(),
        exists,
        degree: 0,
    }
}

/// The distance linked nodes settle at, about.
const SPACING: f32 = 40.0;
const GRAVITY: f32 = 0.05;
/// A cell this many times farther than it is wide acts as one body.
const FAR: f32 = 1.2;

/// A square of the plane: the bodies in it, as one at their center.
struct Cell {
    /// Its middle and half its side.
    x: f32,
    y: f32,
    half: f32,
    /// How many bodies, and their center.
    mass: f32,
    cx: f32,
    cy: f32,
    /// Its four quarters, once it holds more than one body.
    kids: Option<usize>,
}

/// A quadtree over the nodes' positions: its root is cell 0.
struct Tree(Vec<Cell>);

impl Tree {
    fn new(at: &[(f32, f32)]) -> Self {
        let half = at
            .iter()
            .fold(1.0_f32, |m, &(x, y)| m.max(x.abs()).max(y.abs()));
        let mut tree = Tree(vec![cell(0.0, 0.0, half)]);
        for &(x, y) in at {
            tree.insert(0, x, y, 1.0);
        }
        tree
    }

    fn insert(&mut self, i: usize, x: f32, y: f32, mass: f32) {
        let c = &self.0[i];
        let (held, cx, cy, half) = (c.mass, c.cx, c.cy, c.half);
        // A cell too small to split keeps its bodies as one.
        if held > 0.0 && c.kids.is_none() && half > 1e-3 {
            let first = self.0.len();
            for (sx, sy) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
                let (x, y) = (self.0[i].x + sx * half / 2.0, self.0[i].y + sy * half / 2.0);
                self.0.push(cell(x, y, half / 2.0));
            }
            self.0[i].kids = Some(first);
            let quarter = self.quarter(i, cx, cy);
            self.insert(quarter, cx, cy, held);
        }
        let c = &mut self.0[i];
        c.cx = (c.cx * c.mass + x * mass) / (c.mass + mass);
        c.cy = (c.cy * c.mass + y * mass) / (c.mass + mass);
        c.mass += mass;
        if self.0[i].kids.is_some() {
            let quarter = self.quarter(i, x, y);
            self.insert(quarter, x, y, mass);
        }
    }

    /// The quarter of split cell `i` the point is in.
    fn quarter(&self, i: usize, x: f32, y: f32) -> usize {
        let c = &self.0[i];
        c.kids.unwrap_or(i) + usize::from(x > c.x) + 2 * usize::from(y > c.y)
    }

    /// How the bodies of cell `i` push a node at the point.
    fn repulsion(&self, i: usize, x: f32, y: f32) -> (f32, f32) {
        let c = &self.0[i];
        let (dx, dy) = (x - c.cx, y - c.cy);
        let distance2 = dx * dx + dy * dy + 0.01;
        match c.kids {
            Some(first) if c.half * c.half * 4.0 * FAR * FAR > distance2 => (first..first + 4)
                .map(|kid| self.repulsion(kid, x, y))
                .fold((0.0, 0.0), |sum, f| (sum.0 + f.0, sum.1 + f.1)),
            _ => {
                let push = SPACING * SPACING * c.mass / distance2;
                (dx * push, dy * push)
            }
        }
    }
}

fn cell(x: f32, y: f32, half: f32) -> Cell {
    Cell {
        x,
        y,
        half,
        mass: 0.0,
        cx: 0.0,
        cy: 0.0,
        kids: None,
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::vault::Vault;

    fn distance(a: (f32, f32), b: (f32, f32)) -> f32 {
        ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
    }

    #[test]
    fn builds_from_the_index() {
        let root = std::env::temp_dir().join(format!("calliope-graph-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let vault = Vault::open(root).unwrap();
        fs::write(vault.abs("A.md"), "[[B]] [[B]] [[Missing]] [[A]]").unwrap();
        fs::write(vault.abs("B.md"), "[[A]]").unwrap();
        fs::write(vault.abs("C.md"), "alone").unwrap();
        let mut index = Index::default();
        index.sync(&vault).unwrap();

        let graph = Graph::of(&index, false, false);
        let ids: Vec<_> = graph.nodes.iter().map(|n| n.id.as_str()).collect();
        assert_eq!(ids, ["A.md", "B.md"]);
        assert_eq!(graph.edges, [(0, 1)]);

        let graph = Graph::of(&index, true, true);
        let ids: Vec<_> = graph.nodes.iter().map(|n| n.id.as_str()).collect();
        assert_eq!(ids, ["A.md", "B.md", "C.md", "Missing"]);
        assert_eq!(graph.edges, [(0, 1), (0, 3)]);
        assert_eq!(graph.nodes[0].degree, 2);
        assert!(!graph.nodes[3].exists);
    }

    #[test]
    fn linked_nodes_end_closer_than_unlinked_ones() {
        // Two chains of ten nodes, not linked to each other.
        let graph = Graph {
            nodes: (0..20).map(|i| node(&i.to_string(), true)).collect(),
            edges: (0..9)
                .flat_map(|i| [(i, i + 1), (i + 10, i + 11)])
                .collect(),
        };
        let at = graph.layout();
        assert!(at.iter().all(|p| p.0.is_finite() && p.1.is_finite()));
        assert!(distance(at[0], at[1]) < distance(at[0], at[10]));
        // No two nodes on the same spot.
        assert!(distance(at[3], at[4]) > 1.0);
    }

    #[test]
    fn lays_out_5000_nodes() {
        let graph = Graph {
            nodes: (0..5000).map(|i| node(&i.to_string(), true)).collect(),
            edges: (1..5000).map(|i| (i / 3, i)).collect(),
        };
        let started = std::time::Instant::now();
        let at = graph.layout();
        assert!(at.iter().all(|p| p.0.is_finite() && p.1.is_finite()));
        assert!(started.elapsed().as_secs() < 20, "{:?}", started.elapsed());
    }
}

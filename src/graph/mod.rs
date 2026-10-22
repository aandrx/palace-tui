use anyhow::Result;
use petgraph::stable_graph::{NodeIndex, StableGraph};
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct NoteNode {
    pub id: i64,
    pub title: String,
    pub path: PathBuf,
    pub out_degree: usize,
    pub in_degree: usize,
}

impl NoteNode {
    pub fn new(id: i64, title: String, path: PathBuf) -> Self {
        Self {
            id,
            title,
            path,
            out_degree: 0,
            in_degree: 0,
        }
    }

    pub fn total_degree(&self) -> usize {
        self.out_degree + self.in_degree
    }
}

#[derive(Debug, Clone)]
pub struct LinkEdge {
    pub source_id: i64,
    pub target_id: i64,
    pub link_type: String,
}

#[derive(Debug, Clone)]
pub struct NodePosition {
    pub x: f32,
    pub y: f32,
}

pub struct NoteGraph {
    pub graph: StableGraph<NoteNode, LinkEdge>,
    node_index_map: HashMap<i64, NodeIndex>,
}

impl NoteGraph {
    pub fn new() -> Self {
        Self {
            graph: StableGraph::default(),
            node_index_map: HashMap::new(),
        }
    }

    pub fn add_node(&mut self, node: NoteNode) -> NodeIndex {
        let idx = self.graph.add_node(node);
        self.node_index_map.insert(idx.index() as i64, idx);
        idx
    }

    pub fn add_edge(&mut self, source: NodeIndex, target: NodeIndex, edge: LinkEdge) {
        self.graph.add_edge(source, target, edge);
    }

    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    pub fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }

    pub fn get_node(&self, idx: NodeIndex) -> Option<&NoteNode> {
        self.graph.node_weight(idx)
    }

    pub fn get_node_by_id(&self, id: i64) -> Option<NodeIndex> {
        self.node_index_map.get(&id).copied()
    }

    pub fn get_all_nodes(&self) -> impl Iterator<Item = (NodeIndex, &NoteNode)> {
        self.graph
            .node_indices()
            .filter_map(|idx| self.graph.node_weight(idx).map(|node| (idx, node)))
    }

    pub fn get_neighbors(&self, idx: NodeIndex) -> Vec<NodeIndex> {
        self.graph.neighbors(idx).collect()
    }

    pub fn get_outgoing_neighbors(&self, idx: NodeIndex) -> Vec<NodeIndex> {
        self.graph
            .neighbors_directed(idx, petgraph::Direction::Outgoing)
            .collect()
    }

    pub fn get_incoming_neighbors(&self, idx: NodeIndex) -> Vec<NodeIndex> {
        self.graph
            .neighbors_directed(idx, petgraph::Direction::Incoming)
            .collect()
    }

    pub fn get_connected_component(&self, center: NodeIndex, depth: usize) -> Vec<NodeIndex> {
        let mut visited = HashMap::new();
        let mut queue = VecDeque::new();
        queue.push_back((center, 0));
        visited.insert(center, 0);

        while let Some((node, dist)) = queue.pop_front() {
            if dist >= depth {
                continue;
            }
            for neighbor in self.graph.neighbors(node) {
                if !visited.contains_key(&neighbor) {
                    visited.insert(neighbor, dist + 1);
                    queue.push_back((neighbor, dist + 1));
                }
            }
        }

        visited.into_keys().collect()
    }

    pub fn get_subgraph(&self, nodes: &[NodeIndex]) -> StableGraph<&NoteNode, &LinkEdge> {
        let mut subgraph = StableGraph::default();
        let mut index_map: HashMap<NodeIndex, petgraph::graph::NodeIndex> = HashMap::new();

        for &node in nodes {
            if let Some(node_weight) = self.graph.node_weight(node) {
                let new_idx = subgraph.add_node(node_weight);
                index_map.insert(node, new_idx);
            }
        }

        for &node in nodes {
            if let Some(new_src) = index_map.get(&node) {
                for neighbor in self.graph.neighbors(node) {
                    if let Some(new_dst) = index_map.get(&neighbor) {
                        if let Some(edge_weight) = self.graph.find_edge(node, neighbor) {
                            let edge = self.graph.edge_weight(edge_weight).unwrap();
                            subgraph.add_edge(*new_src, *new_dst, edge);
                        }
                    }
                }
            }
        }

        subgraph
    }

    pub fn find_orphans(&self) -> Vec<NodeIndex> {
        self.graph
            .node_indices()
            .filter(|&idx| self.graph.neighbors(idx).count() == 0)
            .collect()
    }

    pub fn nodes_sorted_by_degree(&self) -> Vec<(NodeIndex, usize)> {
        let mut nodes: Vec<_> = self
            .graph
            .node_indices()
            .map(|idx| {
                let degree = self.graph.neighbors(idx).count();
                (idx, degree)
            })
            .collect();
        nodes.sort_by(|a, b| b.1.cmp(&a.1));
        nodes
    }

    pub fn compute_force_directed_layout(
        &self,
        iterations: usize,
        target_width: usize,
        target_height: usize,
    ) -> HashMap<NodeIndex, NodePosition> {
        use std::collections::hash_map::Entry;

        let n = self.graph.node_count();
        if n == 0 {
            return HashMap::new();
        }

        let padding = 5.0;
        let usable_width = (target_width as f32).max(20.0) - 2.0 * padding;
        let usable_height = (target_height as f32).max(10.0) - 2.0 * padding;

        let mut positions: HashMap<NodeIndex, NodePosition> = self
            .graph
            .node_indices()
            .map(|idx| {
                let angle = (idx.index() as f32) * 2.0 * std::f32::consts::PI / (n as f32);
                let radius = usable_width.min(usable_height) * 0.4;
                (
                    idx,
                    NodePosition {
                        x: padding + usable_width / 2.0 + radius * angle.cos(),
                        y: padding + usable_height / 2.0 + radius * angle.sin(),
                    },
                )
            })
            .collect();

        let repulsion = usable_width * usable_height * 0.5;
        let attraction = 0.05;
        let damping = 0.85;
        let center_x = padding + usable_width / 2.0;
        let center_y = padding + usable_height / 2.0;

        let actual_iterations = if n > 200 {
            (iterations as f64 * 200.0 / n as f64).max(20.0) as usize
        } else {
            iterations
        };

        eprintln!(
            "[DEBUG] Layout: {} nodes, {} iterations (requested {})",
            n, actual_iterations, iterations
        );

        for iter in 0..actual_iterations {
            let mut forces: HashMap<NodeIndex, (f32, f32)> = self
                .graph
                .node_indices()
                .map(|idx| (idx, (0.0, 0.0)))
                .collect();

            for i in self.graph.node_indices() {
                for j in self.graph.node_indices() {
                    if i == j {
                        continue;
                    }

                    let pos_i = positions.get(&i).unwrap();
                    let pos_j = positions.get(&j).unwrap();

                    let dx = pos_i.x - pos_j.x;
                    let dy = pos_i.y - pos_j.y;
                    let dist = (dx * dx + dy * dy).sqrt().max(1.0);

                    let force = repulsion / (dist * dist);
                    let fx = (dx / dist) * force;
                    let fy = (dy / dist) * force;

                    if let Entry::Occupied(mut e) = forces.entry(i) {
                        let (fx_sum, fy_sum) = e.get();
                        e.insert((fx_sum + fx, fy_sum + fy));
                    }
                }
            }

            for edge in self.graph.edge_indices() {
                if let (Some(src), Some(dst)) = (
                    self.graph.edge_endpoints(edge).map(|(s, _)| s),
                    self.graph.edge_endpoints(edge).map(|(_, d)| d),
                ) {
                    let pos_src = positions.get(&src).unwrap();
                    let pos_dst = positions.get(&dst).unwrap();

                    let dx = pos_dst.x - pos_src.x;
                    let dy = pos_dst.y - pos_src.y;
                    let dist = (dx * dx + dy * dy).sqrt().max(1.0);

                    let force = dist * attraction;
                    let fx = (dx / dist) * force;
                    let fy = (dy / dist) * force;

                    if let Entry::Occupied(mut e) = forces.entry(src) {
                        let (fx_sum, fy_sum) = e.get();
                        e.insert((fx_sum + fx, fy_sum - fy));
                    }
                    if let Entry::Occupied(mut e) = forces.entry(dst) {
                        let (fx_sum, fy_sum) = e.get();
                        e.insert((fx_sum - fx, fy_sum + fy));
                    }
                }
            }

            for (idx, (fx, fy)) in forces {
                if let Some(pos) = positions.get_mut(&idx) {
                    pos.x += fx * damping;
                    pos.y += fy * damping;

                    let dx = pos.x - center_x;
                    let dy = pos.y - center_y;
                    pos.x -= dx * 0.01;
                    pos.y -= dy * 0.01;
                }
            }

            if iter % 10 == 0 {
                eprintln!("[DEBUG] Layout iteration {}/{}", iter, actual_iterations);
            }
        }

        positions
    }

    pub fn normalize_positions_to_bounds(
        positions: &HashMap<NodeIndex, NodePosition>,
        width: f32,
        height: f32,
        padding: f32,
    ) -> HashMap<NodeIndex, NodePosition> {
        positions
            .iter()
            .map(|(&idx, pos)| {
                (
                    idx,
                    NodePosition {
                        x: pos.x * (width - 2.0 * padding) + padding,
                        y: pos.y * (height - 2.0 * padding) + padding,
                    },
                )
            })
            .collect()
    }
}

impl Default for NoteGraph {
    fn default() -> Self {
        Self::new()
    }
}

pub struct GraphBuilder<'a> {
    graph: NoteGraph,
    db: &'a crate::storage::Database,
}

impl<'a> GraphBuilder<'a> {
    pub fn new(db: &'a crate::storage::Database) -> Self {
        Self {
            graph: NoteGraph::new(),
            db,
        }
    }

    pub fn build(mut self) -> Result<NoteGraph> {
        let notes = self.db.get_all_notes()?;
        let links = self.db.get_all_note_links()?;

        let mut node_ids: HashMap<i64, NodeIndex> = HashMap::new();

        for note in &notes {
            let node = NoteNode::new(
                self.db.get_note_id_by_path(&note.path)?.unwrap_or(0),
                note.metadata.title.clone().unwrap_or_else(|| {
                    note.path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("Untitled")
                        .to_string()
                }),
                note.path.clone(),
            );
            let idx = self.graph.add_node(node);
            if let Some(id) = self.db.get_note_id_by_path(&note.path)? {
                node_ids.insert(id, idx);
            }
        }

        for (source_id, target_id) in links {
            if let (Some(&src_idx), Some(&dst_idx)) =
                (node_ids.get(&source_id), node_ids.get(&target_id))
            {
                self.graph.add_edge(
                    src_idx,
                    dst_idx,
                    LinkEdge {
                        source_id,
                        target_id,
                        link_type: "link".to_string(),
                    },
                );
            }
        }

        let indices: Vec<_> = self.graph.graph.node_indices().collect();
        let degrees: Vec<_> = indices
            .iter()
            .map(|&idx| {
                (
                    idx,
                    self.graph.get_outgoing_neighbors(idx).len(),
                    self.graph.get_incoming_neighbors(idx).len(),
                )
            })
            .collect();

        for (idx, out_deg, in_deg) in degrees {
            if let Some(node) = self.graph.graph.node_weight_mut(idx) {
                node.out_degree = out_deg;
                node.in_degree = in_deg;
            }
        }

        Ok(self.graph)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graph_creation() {
        let graph = NoteGraph::new();
        assert_eq!(graph.node_count(), 0);
        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn test_add_node() {
        let mut graph = NoteGraph::new();
        let node = NoteNode::new(1, "Test".to_string(), PathBuf::from("/test.md"));
        let idx = graph.add_node(node);
        assert_eq!(graph.node_count(), 1);
        assert_eq!(graph.get_node(idx).unwrap().title, "Test");
    }

    #[test]
    fn test_add_edge() {
        let mut graph = NoteGraph::new();
        let node1 = NoteNode::new(1, "Node 1".to_string(), PathBuf::from("/1.md"));
        let node2 = NoteNode::new(2, "Node 2".to_string(), PathBuf::from("/2.md"));
        let idx1 = graph.add_node(node1);
        let idx2 = graph.add_node(node2);

        graph.add_edge(
            idx1,
            idx2,
            LinkEdge {
                source_id: 1,
                target_id: 2,
                link_type: "link".to_string(),
            },
        );

        assert_eq!(graph.edge_count(), 1);
        assert_eq!(graph.get_neighbors(idx1), vec![idx2]);
    }

    #[test]
    fn test_force_directed_layout() {
        let mut graph = NoteGraph::new();
        let node1 = NoteNode::new(1, "Node 1".to_string(), PathBuf::from("/1.md"));
        let node2 = NoteNode::new(2, "Node 2".to_string(), PathBuf::from("/2.md"));
        let idx1 = graph.add_node(node1);
        let idx2 = graph.add_node(node2);
        graph.add_edge(
            idx1,
            idx2,
            LinkEdge {
                source_id: 1,
                target_id: 2,
                link_type: "link".to_string(),
            },
        );

        let positions = graph.compute_force_directed_layout(10, 80, 24);
        assert_eq!(positions.len(), 2);

        for pos in positions.values() {
            assert!(pos.x >= 0.0 && pos.x <= 80.0);
            assert!(pos.y >= 0.0 && pos.y <= 24.0);
        }
    }

    #[test]
    fn test_find_orphans() {
        let mut graph = NoteGraph::new();
        let node1 = NoteNode::new(1, "Node 1".to_string(), PathBuf::from("/1.md"));
        let node2 = NoteNode::new(2, "Node 2".to_string(), PathBuf::from("/2.md"));
        let node3 = NoteNode::new(3, "Node 3".to_string(), PathBuf::from("/3.md"));

        let idx1 = graph.add_node(node1);
        let idx2 = graph.add_node(node2);
        let idx3 = graph.add_node(node3);

        graph.add_edge(
            idx1,
            idx2,
            LinkEdge {
                source_id: 1,
                target_id: 2,
                link_type: "link".to_string(),
            },
        );

        let orphans = graph.find_orphans();
        assert_eq!(orphans.len(), 2);
        assert!(orphans.contains(&idx2));
        assert!(orphans.contains(&idx3));
    }
}

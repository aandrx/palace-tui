use crate::graph::{NodePosition, NoteGraph};
use petgraph::stable_graph::NodeIndex;

#[derive(Debug, Clone)]
pub struct RenderConfig {
    pub width: usize,
    pub height: usize,
    pub node_radius: f32,
    pub show_labels: bool,
    pub selected_idx: Option<NodeIndex>,
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            width: 80,
            height: 24,
            node_radius: 1.0,
            show_labels: true,
            selected_idx: None,
        }
    }
}

pub struct GraphRenderer {
    config: RenderConfig,
}

impl GraphRenderer {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            config: RenderConfig {
                width,
                height,
                ..Default::default()
            },
        }
    }

    pub fn with_config(config: RenderConfig) -> Self {
        Self { config }
    }

    pub fn set_selected(&mut self, idx: Option<NodeIndex>) {
        self.config.selected_idx = idx;
    }

    pub fn render(
        &self,
        graph: &NoteGraph,
        positions: &std::collections::HashMap<NodeIndex, NodePosition>,
    ) -> Vec<String> {
        let mut grid = vec![vec![' '; self.config.width]; self.config.height];

        let edges = self.draw_edges(graph, positions, &mut grid);

        self.draw_nodes(graph, positions, &mut grid);

        let mut lines: Vec<String> = grid
            .iter()
            .map(|row| row.iter().collect::<String>())
            .collect();

        if self.config.show_labels {
            self.draw_labels(graph, positions, &mut lines);
        }

        self.draw_info_panel(graph, &mut lines);

        lines
    }

    fn draw_edges(
        &self,
        graph: &NoteGraph,
        positions: &std::collections::HashMap<NodeIndex, NodePosition>,
        grid: &mut Vec<Vec<char>>,
    ) -> Vec<(NodeIndex, NodeIndex)> {
        let mut drawn_edges = Vec::new();

        for edge_idx in graph.graph.edge_indices() {
            if let Some((src, dst)) = graph.graph.edge_endpoints(edge_idx) {
                if let (Some(pos_src), Some(pos_dst)) = (positions.get(&src), positions.get(&dst)) {
                    let x1 = pos_src.x as i32;
                    let y1 = pos_src.y as i32;
                    let x2 = pos_dst.x as i32;
                    let y2 = pos_dst.y as i32;

                    if x1 == x2 && y1 == y2 {
                        continue;
                    }

                    let points = self.bresenham_line(x1, y1, x2, y2);

                    for (i, &(px, py)) in points.iter().enumerate() {
                        let ux = px as usize;
                        let uy = py as usize;

                        if ux >= self.config.width || uy >= self.config.height {
                            continue;
                        }

                        let is_endpoint = i == 0 || i == points.len() - 1;
                        let char_at_pos = grid[uy][ux];

                        if char_at_pos == ' ' || is_endpoint {
                            let char = if is_endpoint && char_at_pos == ' ' {
                                '·'
                            } else if char_at_pos == ' ' {
                                '·'
                            } else {
                                continue;
                            };
                            grid[uy][ux] = char;
                        }
                    }

                    drawn_edges.push((src, dst));
                }
            }
        }

        drawn_edges
    }

    fn bresenham_line(&self, x0: i32, y0: i32, x1: i32, y1: i32) -> Vec<(i32, i32)> {
        let mut points = Vec::new();

        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;

        let mut x = x0;
        let mut y = y0;

        loop {
            points.push((x, y));

            if x == x1 && y == y1 {
                break;
            }

            let e2 = 2 * err;

            if e2 >= dy {
                if x == x1 {
                    break;
                }
                err += dy;
                x += sx;
            }

            if e2 <= dx {
                if y == y1 {
                    break;
                }
                err += dx;
                y += sy;
            }
        }

        points
    }

    fn draw_nodes(
        &self,
        graph: &NoteGraph,
        positions: &std::collections::HashMap<NodeIndex, NodePosition>,
        grid: &mut Vec<Vec<char>>,
    ) {
        for idx in graph.graph.node_indices() {
            if let (Some(_node), Some(pos)) = (graph.get_node(idx), positions.get(&idx)) {
                let x = (pos.x as usize).min(self.config.width - 1);
                let y = (pos.y as usize).min(self.config.height - 1);

                let neighbors = graph.get_neighbors(idx);
                let is_selected = self.config.selected_idx.map(|s| s == idx).unwrap_or(false);
                let is_orphan = neighbors.is_empty();

                let char = if is_selected {
                    '◉'
                } else if is_orphan {
                    '○'
                } else {
                    '●'
                };

                if y < self.config.height && x < self.config.width {
                    grid[y][x] = char;
                }
            }
        }
    }

    fn draw_labels(
        &self,
        graph: &NoteGraph,
        positions: &std::collections::HashMap<NodeIndex, NodePosition>,
        lines: &mut Vec<String>,
    ) {
        let label_offset = 2;

        let mut labels: Vec<_> = graph
            .graph
            .node_indices()
            .filter_map(|idx| {
                let node = graph.get_node(idx)?;
                let pos = positions.get(&idx)?;
                Some((idx, node, pos.clone()))
            })
            .collect();

        labels.sort_by(|a, b| {
            let y_a = (a.2.y as usize).min(self.config.height.saturating_sub(1));
            let y_b = (b.2.y as usize).min(self.config.height.saturating_sub(1));
            y_a.cmp(&y_b)
        });

        for (idx, node, pos) in labels {
            let x = pos.x as usize + label_offset;
            let y = pos.y as usize;

            if y >= self.config.height || x >= self.config.width {
                continue;
            }

            let max_chars = self.config.width.saturating_sub(x);
            let truncated_title: String = node.title.chars().take(max_chars).collect();
            let is_selected = self.config.selected_idx.map(|s| s == idx).unwrap_or(false);

            let prefix = if is_selected { "> " } else { "  " };
            let label = format!("{}{}", prefix, truncated_title);

            let line_chars: Vec<char> = lines[y].chars().collect();
            let prefix_chars: Vec<char> = label.chars().collect();

            let mut new_line = String::new();
            for (i, c) in line_chars.iter().enumerate() {
                if i >= x && i < x + prefix_chars.len() {
                    let label_idx = i - x;
                    if label_idx < prefix_chars.len() {
                        new_line.push(prefix_chars[label_idx]);
                    } else {
                        new_line.push(*c);
                    }
                } else {
                    new_line.push(*c);
                }
            }

            if new_line.len() < self.config.width {
                new_line.extend(std::iter::repeat(' ').take(self.config.width - new_line.len()));
            }

            lines[y] = new_line.chars().take(self.config.width).collect();
        }
    }

    fn draw_info_panel(&self, graph: &NoteGraph, lines: &mut Vec<String>) {
        let panel_height = 3.min(self.config.height);
        let panel_start = self.config.height.saturating_sub(panel_height);

        if let Some(selected) = self.config.selected_idx {
            if let Some(node) = graph.get_node(selected) {
                let info = format!(
                    " {} | Links: {} out, {} in ",
                    truncate_string(&node.title, 30),
                    node.out_degree,
                    node.in_degree
                );

                if panel_start < lines.len() {
                    let info_chars: Vec<char> = info.chars().take(self.config.width).collect();
                    let line_chars: Vec<char> = lines[panel_start].chars().collect();

                    let mut new_line = String::new();
                    for i in 0..self.config.width {
                        if i < info_chars.len() {
                            new_line.push(info_chars[i]);
                        } else if i < line_chars.len() {
                            new_line.push(line_chars[i]);
                        } else {
                            new_line.push(' ');
                        }
                    }

                    lines[panel_start] = new_line;
                }
            }
        }
    }

    pub fn get_grid_size(&self) -> (usize, usize) {
        (self.config.width, self.config.height)
    }
}

fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

pub fn render_simple(
    graph: &NoteGraph,
    positions: &std::collections::HashMap<NodeIndex, NodePosition>,
    width: usize,
    height: usize,
) -> String {
    let renderer = GraphRenderer::new(width, height);
    renderer.render(graph, positions).join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{LinkEdge, NoteNode};
    use std::collections::HashMap;
    use std::path::PathBuf;

    #[test]
    fn test_renderer_creation() {
        let renderer = GraphRenderer::new(80, 24);
        assert_eq!(renderer.get_grid_size(), (80, 24));
    }

    #[test]
    fn test_truncate_string() {
        assert_eq!(truncate_string("hello", 10), "hello");
        assert_eq!(truncate_string("hello world", 8), "hello...");
        assert_eq!(truncate_string("hi", 5), "hi");
    }

    #[test]
    fn test_render_empty_graph() {
        let graph = NoteGraph::new();
        let positions = HashMap::new();
        let renderer = GraphRenderer::new(40, 10);
        let output = renderer.render(&graph, &positions);

        assert_eq!(output.len(), 10);
        assert!(output.iter().all(|line| line.len() == 40));
    }
}

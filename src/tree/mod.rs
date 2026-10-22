use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub enum TreeNode {
    Folder {
        name: String,
        children: Vec<TreeNode>,
        note_count: usize,
    },
    File {
        name: String,
        path: PathBuf,
        link_count: usize,
    },
}

impl TreeNode {
    pub fn from_paths(paths: &[(PathBuf, usize)]) -> Vec<TreeNode> {
        let mut tree: HashMap<String, TreeNode> = HashMap::new();

        for (path, link_count) in paths {
            let components: Vec<_> = path
                .components()
                .filter_map(|c| c.as_os_str().to_str())
                .collect();

            if components.is_empty() {
                continue;
            }

            let file_name = components.last().unwrap();
            let parent_path = if components.len() > 1 {
                components[..components.len() - 1].join("/")
            } else {
                String::new()
            };

            let file_node = TreeNode::File {
                name: file_name.to_string(),
                path: path.clone(),
                link_count: *link_count,
            };

            if parent_path.is_empty() {
                tree.insert(file_name.to_string(), file_node);
            } else {
                let folder_name = parent_path.clone();
                let folder = tree
                    .entry(folder_name.clone())
                    .or_insert_with(|| TreeNode::Folder {
                        name: folder_name,
                        children: Vec::new(),
                        note_count: 0,
                    });

                if let TreeNode::Folder {
                    children,
                    note_count,
                    ..
                } = folder
                {
                    children.push(file_node);
                    *note_count += 1;
                }
            }
        }

        let mut root_nodes: Vec<TreeNode> = tree.into_values().collect();
        root_nodes.sort_by(|a, b| {
            let name_a = match a {
                TreeNode::Folder { name, .. } => name,
                TreeNode::File { name, .. } => name,
            };
            let name_b = match b {
                TreeNode::Folder { name, .. } => name,
                TreeNode::File { name, .. } => name,
            };
            name_a.cmp(name_b)
        });

        root_nodes
    }

    pub fn from_flat_paths(paths: &[(PathBuf, usize)]) -> Vec<TreeNode> {
        let mut files: Vec<_> = paths
            .iter()
            .map(|(path, link_count)| TreeNode::File {
                name: path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Unknown")
                    .to_string(),
                path: path.clone(),
                link_count: *link_count,
            })
            .collect();

        files.sort_by(|a, b| {
            let name_a = match a {
                TreeNode::File { name, .. } => name,
                _ => "",
            };
            let name_b = match b {
                TreeNode::File { name, .. } => name,
                _ => "",
            };
            name_a.cmp(name_b)
        });

        files
    }
}

pub fn render_tree(
    nodes: &[TreeNode],
    width: usize,
    max_depth: usize,
    selected_idx: Option<usize>,
) -> Vec<String> {
    let mut lines = Vec::new();
    let mut idx = 0;

    for node in nodes {
        render_node(
            node,
            &mut lines,
            "",
            true,
            &mut idx,
            width,
            max_depth,
            selected_idx,
        );
    }

    lines
}

fn render_node(
    node: &TreeNode,
    lines: &mut Vec<String>,
    prefix: &str,
    is_last: bool,
    idx: &mut usize,
    width: usize,
    max_depth: usize,
    selected_idx: Option<usize>,
) {
    let current_idx = *idx;
    *idx += 1;

    let connector = if is_last { "└── " } else { "├── " };
    let is_selected = selected_idx.map(|s| s == current_idx).unwrap_or(false);
    let prefix_char = if is_selected { "▸ " } else { "  " };

    match node {
        TreeNode::Folder {
            name,
            children,
            note_count,
        } => {
            let line = format!(
                "{}{}{}📁 {} ({} notes)",
                prefix, connector, prefix_char, name, note_count
            );
            lines.push(truncate_line(&line, width));

            let child_prefix = if is_last {
                format!("{}    ", prefix)
            } else {
                format!("{}│   ", prefix)
            };

            for (i, child) in children.iter().enumerate() {
                let is_last_child = i == children.len() - 1;
                render_node(
                    child,
                    lines,
                    &child_prefix,
                    is_last_child,
                    idx,
                    width,
                    max_depth,
                    selected_idx,
                );
            }
        }
        TreeNode::File {
            name,
            path: _,
            link_count,
        } => {
            let link_info = if *link_count > 0 {
                format!(" [{} links]", link_count)
            } else {
                String::new()
            };

            let line = format!(
                "{}{}{}📄 {}{}",
                prefix, connector, prefix_char, name, link_info
            );
            lines.push(truncate_line(&line, width));
        }
    }
}

fn truncate_line(line: &str, max_width: usize) -> String {
    if line.chars().count() <= max_width {
        line.to_string()
    } else {
        let truncated: String = line.chars().take(max_width - 3).collect();
        format!("{}...", truncated)
    }
}

pub struct NoteTree {
    pub nodes: Vec<TreeNode>,
    pub total_notes: usize,
    pub total_links: usize,
}

impl NoteTree {
    pub fn from_paths(paths: &[(PathBuf, usize)]) -> Self {
        let total_notes = paths.len();
        let total_links: usize = paths.iter().map(|(_, links)| links).sum();

        let nodes = TreeNode::from_paths(paths);

        Self {
            nodes,
            total_notes,
            total_links,
        }
    }

    pub fn render(&self, width: usize, selected_idx: Option<usize>) -> Vec<String> {
        render_tree(&self.nodes, width, 10, selected_idx)
    }

    pub fn get_all_files(&self) -> Vec<(usize, &str, &PathBuf, usize)> {
        let mut files = Vec::new();
        let mut idx = 0;
        self.collect_files(&self.nodes, &mut files, &mut idx);
        files
    }

    fn collect_files<'a>(
        &'a self,
        nodes: &'a [TreeNode],
        files: &mut Vec<(usize, &'a str, &'a PathBuf, usize)>,
        idx: &mut usize,
    ) {
        for node in nodes {
            match node {
                TreeNode::Folder { children, .. } => {
                    self.collect_files(children, files, idx);
                }
                TreeNode::File {
                    name,
                    path,
                    link_count,
                } => {
                    files.push((*idx, name, path, *link_count));
                    *idx += 1;
                }
            }
        }
    }
}

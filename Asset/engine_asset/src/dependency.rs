use std::{
    collections::{HashMap, HashSet, VecDeque},
    fs,
    path::Path,
};

use crate::{
    error::{AssetError, AssetResult},
    id::AssetId,
};

#[derive(Clone, Debug, Default)]
pub struct DependencyGraph {
    direct: HashMap<AssetId, Vec<AssetId>>,
    reverse: HashMap<AssetId, Vec<AssetId>>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DependencyGraphReport {
    pub assets: Vec<AssetId>,
    pub edges: Vec<DependencyEdge>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DependencyScopeReport {
    pub root: AssetId,
    pub direct_dependencies: Vec<AssetId>,
    pub transitive_dependencies: Vec<AssetId>,
    pub direct_dependents: Vec<AssetId>,
    pub transitive_dependents: Vec<AssetId>,
    pub graph: DependencyGraphReport,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DependencyEdge {
    pub asset: AssetId,
    pub dependency: AssetId,
}

impl DependencyGraphReport {
    pub fn to_text(&self) -> String {
        let mut assets = self.assets.clone();
        assets.sort();
        let mut edges = self.edges.clone();
        edges.sort_by_key(|edge| (edge.asset, edge.dependency));
        let mut lines = vec![
            "NGA_DEPENDENCY_GRAPH_V1".to_owned(),
            format!("assets={}", assets.len()),
        ];
        lines.extend(
            assets
                .into_iter()
                .map(|asset| format!("asset|{}", asset.raw())),
        );
        lines.push(format!("edges={}", edges.len()));
        lines.extend(
            edges
                .into_iter()
                .map(|edge| format!("edge|{}|{}", edge.asset.raw(), edge.dependency.raw())),
        );
        lines.join("\n")
    }

    pub fn to_dot(&self) -> String {
        let mut assets = self.assets.clone();
        assets.sort();
        let mut edges = self.edges.clone();
        edges.sort_by_key(|edge| (edge.asset, edge.dependency));
        let mut lines = vec!["digraph AssetDependencies {".to_owned()];
        for asset in assets {
            lines.push(format!("  \"{}\";", asset.raw()));
        }
        for edge in edges {
            lines.push(format!(
                "  \"{}\" -> \"{}\";",
                edge.asset.raw(),
                edge.dependency.raw()
            ));
        }
        lines.push("}".to_owned());
        lines.join("\n")
    }

    pub fn to_json(&self) -> String {
        let mut assets = self.assets.clone();
        assets.sort();
        let mut edges = self.edges.clone();
        edges.sort_by_key(|edge| (edge.asset, edge.dependency));
        let assets = assets
            .into_iter()
            .map(|asset| format!("\"{}\"", asset.raw()))
            .collect::<Vec<_>>()
            .join(",");
        let edges = edges
            .into_iter()
            .map(|edge| {
                format!(
                    "{{\"asset\":\"{}\",\"dependency\":\"{}\"}}",
                    edge.asset.raw(),
                    edge.dependency.raw()
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        format!("{{\"version\":1,\"assets\":[{assets}],\"edges\":[{edges}]}}")
    }

    pub fn to_html(&self) -> String {
        self.to_html_with_labels(std::iter::empty::<(AssetId, String)>())
    }

    pub fn to_html_with_labels(
        &self,
        labels: impl IntoIterator<Item = (AssetId, String)>,
    ) -> String {
        dependency_report_to_html(
            DependencyHtmlKind::Graph,
            self,
            None,
            labels.into_iter().collect(),
        )
    }

    pub fn save_text(&self, path: impl AsRef<Path>) -> AssetResult<()> {
        let path = path.as_ref();
        fs::write(path, self.to_text())
            .map_err(|error| dependency_report_io_error("write", path, error))
    }

    pub fn save_dot(&self, path: impl AsRef<Path>) -> AssetResult<()> {
        let path = path.as_ref();
        fs::write(path, self.to_dot())
            .map_err(|error| dependency_report_io_error("write", path, error))
    }

    pub fn save_json(&self, path: impl AsRef<Path>) -> AssetResult<()> {
        let path = path.as_ref();
        fs::write(path, self.to_json())
            .map_err(|error| dependency_report_io_error("write", path, error))
    }

    pub fn save_html(&self, path: impl AsRef<Path>) -> AssetResult<()> {
        let path = path.as_ref();
        fs::write(path, self.to_html())
            .map_err(|error| dependency_report_io_error("write", path, error))
    }
}

impl DependencyScopeReport {
    pub fn to_text(&self) -> String {
        let mut lines = vec![
            "NGA_DEPENDENCY_SCOPE_V1".to_owned(),
            format!("root={}", self.root.raw()),
            format_asset_list("direct_dependencies", &self.direct_dependencies),
            format_asset_list("transitive_dependencies", &self.transitive_dependencies),
            format_asset_list("direct_dependents", &self.direct_dependents),
            format_asset_list("transitive_dependents", &self.transitive_dependents),
            "subgraph".to_owned(),
        ];
        lines.push(self.graph.to_text());
        lines.join("\n")
    }

    pub fn to_dot(&self) -> String {
        self.graph.to_dot()
    }

    pub fn to_json(&self) -> String {
        format!(
            "{{\"version\":1,\"root\":\"{}\",\"direct_dependencies\":[{}],\"transitive_dependencies\":[{}],\"direct_dependents\":[{}],\"transitive_dependents\":[{}],\"graph\":{}}}",
            self.root.raw(),
            json_asset_list(&self.direct_dependencies),
            json_asset_list(&self.transitive_dependencies),
            json_asset_list(&self.direct_dependents),
            json_asset_list(&self.transitive_dependents),
            self.graph.to_json()
        )
    }

    pub fn to_html(&self) -> String {
        self.to_html_with_labels(std::iter::empty::<(AssetId, String)>())
    }

    pub fn to_html_with_labels(
        &self,
        labels: impl IntoIterator<Item = (AssetId, String)>,
    ) -> String {
        dependency_report_to_html(
            DependencyHtmlKind::Scope(self),
            &self.graph,
            Some(self.root),
            labels.into_iter().collect(),
        )
    }

    pub fn save_text(&self, path: impl AsRef<Path>) -> AssetResult<()> {
        let path = path.as_ref();
        fs::write(path, self.to_text())
            .map_err(|error| dependency_report_io_error("write", path, error))
    }

    pub fn save_dot(&self, path: impl AsRef<Path>) -> AssetResult<()> {
        let path = path.as_ref();
        fs::write(path, self.to_dot())
            .map_err(|error| dependency_report_io_error("write", path, error))
    }

    pub fn save_json(&self, path: impl AsRef<Path>) -> AssetResult<()> {
        let path = path.as_ref();
        fs::write(path, self.to_json())
            .map_err(|error| dependency_report_io_error("write", path, error))
    }

    pub fn save_html(&self, path: impl AsRef<Path>) -> AssetResult<()> {
        let path = path.as_ref();
        fs::write(path, self.to_html())
            .map_err(|error| dependency_report_io_error("write", path, error))
    }
}

impl DependencyGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_asset(&mut self, id: AssetId) {
        self.direct.entry(id).or_default();
    }

    pub fn set_dependencies(&mut self, id: AssetId, dependencies: Vec<AssetId>) {
        if let Some(old) = self.direct.insert(id, dependencies.clone()) {
            for dependency in old {
                if let Some(reverse) = self.reverse.get_mut(&dependency) {
                    reverse.retain(|parent| *parent != id);
                }
            }
        }
        for dependency in dependencies {
            let reverse = self.reverse.entry(dependency).or_default();
            if !reverse.contains(&id) {
                reverse.push(id);
            }
        }
    }

    pub fn add_dependency(&mut self, id: AssetId, dependency: AssetId) {
        let direct = self.direct.entry(id).or_default();
        if !direct.contains(&dependency) {
            direct.push(dependency);
        }
        let reverse = self.reverse.entry(dependency).or_default();
        if !reverse.contains(&id) {
            reverse.push(id);
        }
    }

    pub fn direct_dependencies(&self, id: AssetId) -> &[AssetId] {
        self.direct.get(&id).map(Vec::as_slice).unwrap_or(&[])
    }

    pub fn reverse_dependencies(&self, id: AssetId) -> &[AssetId] {
        self.reverse.get(&id).map(Vec::as_slice).unwrap_or(&[])
    }

    pub fn direct_dependents(&self, id: AssetId) -> Vec<AssetId> {
        let mut dependents = self.reverse_dependencies(id).to_vec();
        dependents.sort();
        dependents
    }

    pub fn transitive_dependencies(&self, id: AssetId) -> Vec<AssetId> {
        let mut visited = HashSet::new();
        let mut ordered = Vec::new();
        self.visit_transitive(id, &mut visited, &mut ordered);
        ordered
    }

    pub fn transitive_dependents(&self, id: AssetId) -> Vec<AssetId> {
        let mut visited = HashSet::new();
        visited.insert(id);
        let mut ordered = Vec::new();
        let mut queue = VecDeque::new();

        for dependent in self.direct_dependents(id) {
            queue.push_back(dependent);
        }

        while let Some(dependent) = queue.pop_front() {
            if !visited.insert(dependent) {
                continue;
            }
            ordered.push(dependent);
            for next in self.direct_dependents(dependent) {
                queue.push_back(next);
            }
        }

        ordered
    }

    pub fn topological_order(&self, root: AssetId) -> Result<Vec<AssetId>, AssetError> {
        let mut visiting = HashSet::new();
        let mut visited = HashSet::new();
        let mut ordered = Vec::new();
        self.visit_topological(root, &mut visiting, &mut visited, &mut ordered)?;
        Ok(ordered)
    }

    pub fn has_cycle_from(&self, root: AssetId) -> bool {
        self.topological_order(root).is_err()
    }

    pub fn report(&self) -> DependencyGraphReport {
        let mut assets = self.direct.keys().copied().collect::<Vec<_>>();
        for dependency in self.reverse.keys() {
            if !assets.contains(dependency) {
                assets.push(*dependency);
            }
        }
        assets.sort();
        let mut edges = self
            .direct
            .iter()
            .flat_map(|(asset, dependencies)| {
                dependencies.iter().map(|dependency| DependencyEdge {
                    asset: *asset,
                    dependency: *dependency,
                })
            })
            .collect::<Vec<_>>();
        edges.sort_by_key(|edge| (edge.asset, edge.dependency));
        DependencyGraphReport { assets, edges }
    }

    pub fn scoped_report(&self, root: AssetId) -> AssetResult<DependencyScopeReport> {
        if !self.contains_asset(root) {
            return Err(AssetError::AssetNotFound { id: root });
        }
        let mut direct_dependencies = self.direct_dependencies(root).to_vec();
        direct_dependencies.sort();
        let mut transitive_dependencies = self.transitive_dependencies(root);
        transitive_dependencies.sort();
        let direct_dependents = self.direct_dependents(root);
        let mut transitive_dependents = self.transitive_dependents(root);
        transitive_dependents.sort();

        let mut scoped_assets = HashSet::new();
        scoped_assets.insert(root);
        scoped_assets.extend(transitive_dependencies.iter().copied());
        scoped_assets.extend(transitive_dependents.iter().copied());
        scoped_assets.extend(direct_dependencies.iter().copied());
        scoped_assets.extend(direct_dependents.iter().copied());

        let mut assets = scoped_assets.iter().copied().collect::<Vec<_>>();
        assets.sort();
        let mut edges = self
            .direct
            .iter()
            .filter(|(asset, _)| scoped_assets.contains(asset))
            .flat_map(|(asset, dependencies)| {
                dependencies
                    .iter()
                    .filter(|dependency| scoped_assets.contains(dependency))
                    .map(|dependency| DependencyEdge {
                        asset: *asset,
                        dependency: *dependency,
                    })
            })
            .collect::<Vec<_>>();
        edges.sort_by_key(|edge| (edge.asset, edge.dependency));

        Ok(DependencyScopeReport {
            root,
            direct_dependencies,
            transitive_dependencies,
            direct_dependents,
            transitive_dependents,
            graph: DependencyGraphReport { assets, edges },
        })
    }

    fn contains_asset(&self, id: AssetId) -> bool {
        self.direct.contains_key(&id) || self.reverse.contains_key(&id)
    }

    fn visit_transitive(
        &self,
        id: AssetId,
        visited: &mut HashSet<AssetId>,
        ordered: &mut Vec<AssetId>,
    ) {
        for dependency in self.direct_dependencies(id) {
            if visited.insert(*dependency) {
                ordered.push(*dependency);
                self.visit_transitive(*dependency, visited, ordered);
            }
        }
    }

    fn visit_topological(
        &self,
        id: AssetId,
        visiting: &mut HashSet<AssetId>,
        visited: &mut HashSet<AssetId>,
        ordered: &mut Vec<AssetId>,
    ) -> Result<(), AssetError> {
        if visited.contains(&id) {
            return Ok(());
        }
        if !visiting.insert(id) {
            return Err(AssetError::CyclicDependency);
        }
        for dependency in self.direct_dependencies(id) {
            self.visit_topological(*dependency, visiting, visited, ordered)?;
        }
        visiting.remove(&id);
        visited.insert(id);
        ordered.push(id);
        Ok(())
    }
}

fn format_asset_list(label: &str, assets: &[AssetId]) -> String {
    format!(
        "{label}={}",
        assets
            .iter()
            .map(|asset| asset.raw().to_string())
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn json_asset_list(assets: &[AssetId]) -> String {
    assets
        .iter()
        .map(|asset| format!("\"{}\"", asset.raw()))
        .collect::<Vec<_>>()
        .join(",")
}

#[derive(Clone, Copy)]
enum DependencyHtmlKind<'a> {
    Graph,
    Scope(&'a DependencyScopeReport),
}

fn dependency_report_to_html(
    kind: DependencyHtmlKind<'_>,
    graph: &DependencyGraphReport,
    root: Option<AssetId>,
    labels: HashMap<AssetId, String>,
) -> String {
    let mut assets = graph.assets.clone();
    assets.sort();
    let mut edges = graph.edges.clone();
    edges.sort_by_key(|edge| (edge.asset, edge.dependency));
    let mut incoming = HashMap::<AssetId, usize>::new();
    let mut outgoing = HashMap::<AssetId, usize>::new();
    for edge in &edges {
        *outgoing.entry(edge.asset).or_default() += 1;
        *incoming.entry(edge.dependency).or_default() += 1;
    }

    let title = match kind {
        DependencyHtmlKind::Graph => "Asset Dependency Graph",
        DependencyHtmlKind::Scope(_) => "Asset Dependency Scope",
    };
    let root_attr = root
        .map(|asset| format!(" data-root=\"{}\"", asset.raw()))
        .unwrap_or_default();
    let mut html = String::new();
    html.push_str("<!doctype html>\n");
    html.push_str("<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n");
    html.push_str("<meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\n");
    html.push_str("<title>");
    push_html_escaped(&mut html, title);
    html.push_str("</title>\n");
    html.push_str("<style>");
    html.push_str(
        "body{margin:0;font:14px/1.45 system-ui,-apple-system,Segoe UI,sans-serif;color:#1f2937;background:#f7f7f4}main{max-width:1120px;margin:0 auto;padding:24px}h1{font-size:24px;margin:0 0 12px}h2{font-size:16px;margin:28px 0 10px}.summary{display:flex;gap:8px;flex-wrap:wrap;margin:0}.summary span,.pill{border:1px solid #d7d7d0;background:#fff;padding:4px 8px;border-radius:6px}.panel{background:#fff;border:1px solid #d7d7d0;border-radius:8px;overflow:hidden}table{width:100%;border-collapse:collapse}th,td{text-align:left;padding:8px 10px;border-bottom:1px solid #ecece7;vertical-align:top}th{font-size:12px;text-transform:uppercase;color:#5f6b7a;background:#fafaf8}tr:last-child td{border-bottom:0}code{font-family:ui-monospace,SFMono-Regular,Consolas,monospace;font-size:12px}.muted{color:#6b7280}.adjacency{display:grid;grid-template-columns:repeat(auto-fit,minmax(260px,1fr));gap:10px}.node{background:#fff;border:1px solid #d7d7d0;border-radius:8px;padding:10px}.node h3{font-size:13px;margin:0 0 6px;word-break:break-all}.deps{margin:8px 0 0;padding-left:18px}.empty{padding:12px;color:#6b7280}",
    );
    html.push_str("</style>\n</head>\n");
    html.push_str("<body");
    html.push_str(&root_attr);
    html.push_str(">\n<main>\n<header>\n<h1>");
    push_html_escaped(&mut html, title);
    html.push_str("</h1>\n<p class=\"summary\"><span>Assets: ");
    html.push_str(&assets.len().to_string());
    html.push_str("</span><span>Edges: ");
    html.push_str(&edges.len().to_string());
    html.push_str("</span>");
    if let Some(root) = root {
        html.push_str("<span>Root: <code>");
        html.push_str(&root.raw().to_string());
        html.push_str("</code></span>");
    }
    html.push_str("</p>\n</header>\n");

    if let DependencyHtmlKind::Scope(scope) = kind {
        html.push_str("<section aria-labelledby=\"scope-heading\">\n<h2 id=\"scope-heading\">Scope</h2>\n<div class=\"panel\"><table><thead><tr><th>Set</th><th>Assets</th></tr></thead><tbody>\n");
        push_scope_row(
            &mut html,
            "Direct dependencies",
            &scope.direct_dependencies,
            &labels,
        );
        push_scope_row(
            &mut html,
            "Transitive dependencies",
            &scope.transitive_dependencies,
            &labels,
        );
        push_scope_row(
            &mut html,
            "Direct dependents",
            &scope.direct_dependents,
            &labels,
        );
        push_scope_row(
            &mut html,
            "Transitive dependents",
            &scope.transitive_dependents,
            &labels,
        );
        html.push_str("</tbody></table></div>\n</section>\n");
    }

    html.push_str("<section aria-labelledby=\"assets-heading\">\n<h2 id=\"assets-heading\">Assets</h2>\n<div class=\"panel\"><table><thead><tr><th>Asset</th><th>Label</th><th>Outgoing</th><th>Incoming</th></tr></thead><tbody>\n");
    if assets.is_empty() {
        html.push_str("<tr><td colspan=\"4\" class=\"empty\">No assets</td></tr>\n");
    } else {
        for asset in &assets {
            html.push_str("<tr data-asset=\"");
            html.push_str(&asset.raw().to_string());
            html.push_str("\"><td><code>");
            html.push_str(&asset.raw().to_string());
            html.push_str("</code></td><td>");
            push_asset_label(&mut html, *asset, &labels);
            html.push_str("</td><td>");
            html.push_str(&outgoing.get(asset).copied().unwrap_or(0).to_string());
            html.push_str("</td><td>");
            html.push_str(&incoming.get(asset).copied().unwrap_or(0).to_string());
            html.push_str("</td></tr>\n");
        }
    }
    html.push_str("</tbody></table></div>\n</section>\n");

    html.push_str("<section aria-labelledby=\"edges-heading\">\n<h2 id=\"edges-heading\">Edges</h2>\n<div class=\"panel\"><table><thead><tr><th>Asset</th><th>Dependency</th></tr></thead><tbody>\n");
    if edges.is_empty() {
        html.push_str("<tr><td colspan=\"2\" class=\"empty\">No edges</td></tr>\n");
    } else {
        for edge in &edges {
            html.push_str("<tr data-edge=\"");
            html.push_str(&edge.asset.raw().to_string());
            html.push_str("-");
            html.push_str(&edge.dependency.raw().to_string());
            html.push_str("\"><td>");
            push_asset_reference(&mut html, edge.asset, &labels);
            html.push_str("</td><td>");
            push_asset_reference(&mut html, edge.dependency, &labels);
            html.push_str("</td></tr>\n");
        }
    }
    html.push_str("</tbody></table></div>\n</section>\n");

    html.push_str("<section aria-labelledby=\"adjacency-heading\">\n<h2 id=\"adjacency-heading\">Adjacency</h2>\n<div class=\"adjacency\">\n");
    if assets.is_empty() {
        html.push_str("<div class=\"node empty\">No assets</div>\n");
    } else {
        for asset in &assets {
            html.push_str("<article class=\"node\" data-node=\"");
            html.push_str(&asset.raw().to_string());
            html.push_str("\"><h3>");
            push_asset_reference(&mut html, *asset, &labels);
            html.push_str("</h3>");
            let dependencies = edges
                .iter()
                .filter(|edge| edge.asset == *asset)
                .map(|edge| edge.dependency)
                .collect::<Vec<_>>();
            if dependencies.is_empty() {
                html.push_str("<p class=\"muted\">No dependencies</p>");
            } else {
                html.push_str("<ol class=\"deps\">");
                for dependency in dependencies {
                    html.push_str("<li>");
                    push_asset_reference(&mut html, dependency, &labels);
                    html.push_str("</li>");
                }
                html.push_str("</ol>");
            }
            html.push_str("</article>\n");
        }
    }
    html.push_str("</div>\n</section>\n");
    html.push_str("</main>\n</body>\n</html>\n");
    html
}

fn push_scope_row(
    html: &mut String,
    label: &str,
    assets: &[AssetId],
    labels: &HashMap<AssetId, String>,
) {
    html.push_str("<tr><td>");
    push_html_escaped(html, label);
    html.push_str("</td><td>");
    if assets.is_empty() {
        html.push_str("<span class=\"muted\">None</span>");
    } else {
        let mut sorted = assets.to_vec();
        sorted.sort();
        for (index, asset) in sorted.iter().enumerate() {
            if index > 0 {
                html.push_str("<br>");
            }
            push_asset_reference(html, *asset, labels);
        }
    }
    html.push_str("</td></tr>\n");
}

fn push_asset_reference(html: &mut String, asset: AssetId, labels: &HashMap<AssetId, String>) {
    html.push_str("<code>");
    html.push_str(&asset.raw().to_string());
    html.push_str("</code>");
    if labels.contains_key(&asset) {
        html.push_str(" <span class=\"muted\">");
        push_asset_label(html, asset, labels);
        html.push_str("</span>");
    }
}

fn push_asset_label(html: &mut String, asset: AssetId, labels: &HashMap<AssetId, String>) {
    if let Some(label) = labels.get(&asset) {
        push_html_escaped(html, label);
    } else {
        html.push_str("<span class=\"muted\">unlabeled</span>");
    }
}

fn push_html_escaped(html: &mut String, value: &str) {
    for character in value.chars() {
        match character {
            '&' => html.push_str("&amp;"),
            '<' => html.push_str("&lt;"),
            '>' => html.push_str("&gt;"),
            '"' => html.push_str("&quot;"),
            '\'' => html.push_str("&#39;"),
            _ => html.push(character),
        }
    }
}

fn dependency_report_io_error(action: &str, path: &Path, error: std::io::Error) -> AssetError {
    AssetError::Io {
        message: format!(
            "failed to {action} dependency report `{}`: {error}",
            path.display()
        ),
    }
}

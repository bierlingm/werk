use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use werk_core::tension::Tension;
use werk_core::tree::Forest;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ScopeKind {
    Tension,
    Subtree,
    Space,
    Query,
    Union,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScopeSpec {
    pub kind: ScopeKind,
    #[serde(default)]
    pub root: Option<String>,
    #[serde(default)]
    pub depth: Option<usize>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub members: Option<Vec<ScopeSpec>>,
}

#[derive(Debug, Clone)]
pub struct Scope {
    pub kind: ScopeKind,
    pub root: Option<String>,
    pub depth: Option<usize>,
    pub name: Option<String>,
    pub status: Option<String>,
    pub members: Vec<Scope>,
    pub at: Option<DateTime<Utc>>,
}

impl Scope {
    pub fn canonical(&self) -> String {
        match self.kind {
            ScopeKind::Tension => format!("#{}", self.root.clone().unwrap_or_default()),
            ScopeKind::Subtree => format!(
                "#{}~d{}",
                self.root.clone().unwrap_or_default(),
                self.depth.unwrap_or(1)
            ),
            ScopeKind::Space => format!(
                "space:{}",
                self.name.clone().unwrap_or_else(|| "active".into())
            ),
            ScopeKind::Query => format!(
                "query:{}",
                self.status.clone().unwrap_or_else(|| "active".into())
            ),
            ScopeKind::Union => {
                let parts: Vec<String> = self.members.iter().map(|s| s.canonical()).collect();
                format!("union({})", parts.join(","))
            }
        }
    }
}

impl ScopeSpec {
    pub fn into_scope(self, root_override: Option<String>, at: Option<DateTime<Utc>>) -> Scope {
        let root = root_override.or(self.root);
        let members = self
            .members
            .unwrap_or_default()
            .into_iter()
            .map(|member| member.into_scope(None, None))
            .collect();
        Scope {
            kind: self.kind,
            root,
            depth: self.depth,
            name: self.name,
            status: self.status,
            members,
            at,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ResolvedScope {
    pub scope: Scope,
    pub tensions: Vec<Tension>,
    pub forest: Forest,
}

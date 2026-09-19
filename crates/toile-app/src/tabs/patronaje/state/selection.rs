use std::collections::BTreeSet;

use toile_engine::draft::{PointKey, SeamKey};

/// What the inspector is pointed at.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Selection {
    /// The piece as a whole.
    #[default]
    None,
    /// Nodes of it, in key order.
    Points(BTreeSet<PointKey>),
    /// The tract leaving one node.
    Edge(PointKey),
    /// One seam of the product, chosen on the whole of it.
    Seam(SeamKey),
}

impl Selection {
    /// The selection one node on its own makes.
    pub fn point(key: PointKey) -> Selection {
        Selection::Points(BTreeSet::from([key]))
    }

    /// The set of nodes chosen, when nodes are what is chosen.
    pub fn chosen(&self) -> Option<&BTreeSet<PointKey>> {
        match self {
            Selection::Points(keys) => Some(keys),
            Selection::None | Selection::Edge(_) | Selection::Seam(_) => None,
        }
    }

    /// The nodes chosen, in key order; nothing when none are.
    pub fn points(&self) -> impl Iterator<Item = PointKey> + '_ {
        self.chosen().into_iter().flatten().copied()
    }

    /// How many nodes are chosen.
    pub fn count(&self) -> usize {
        self.chosen().map_or(0, BTreeSet::len)
    }

    /// The one node chosen, when exactly one is.
    pub fn only(&self) -> Option<PointKey> {
        let keys = self.chosen()?;
        match keys.len() {
            1 => keys.first().copied(),
            _ => None,
        }
    }

    /// Whether `key` is one of the nodes chosen.
    pub fn holds(&self, key: PointKey) -> bool {
        self.chosen().is_some_and(|keys| keys.contains(&key))
    }

    /// The node the chosen tract leaves, when a tract is chosen.
    pub fn edge(&self) -> Option<PointKey> {
        match self {
            Selection::Edge(key) => Some(*key),
            Selection::None | Selection::Points(_) | Selection::Seam(_) => None,
        }
    }

    /// The seam chosen, when a seam is what is chosen.
    pub fn seam(&self) -> Option<SeamKey> {
        match self {
            Selection::Seam(key) => Some(*key),
            Selection::None | Selection::Points(_) | Selection::Edge(_) => None,
        }
    }
}

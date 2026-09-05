mod apply;
mod coalesce;
mod history;
mod kind;

pub use coalesce::Coalesced;
pub use history::History;
pub use kind::{Applied, ChangeClass, Command};

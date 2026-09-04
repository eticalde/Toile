use std::collections::{BTreeMap, BTreeSet};

use super::eval::EvalError;

/// A pattern variable as the ordering sees it: its name and what it reads.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Dependency<'a> {
    /// The variable's own name.
    pub name: &'a str,
    /// Every name its binding mentions, measurements included.
    pub reads: BTreeSet<&'a str>,
}

/// Positions of `variables` in an order where each follows what it reads.
///
/// A name no variable defines is a measurement: it is resolved before any
/// variable is, so it places no edge. A name defined twice keeps its first
/// definition; rejecting the duplicate belongs to whoever owns the names.
///
/// # Errors
/// `EvalError::Cycle`, naming one cycle, when variables depend on each other.
pub fn evaluation_order(variables: &[Dependency<'_>]) -> Result<Vec<usize>, EvalError> {
    let mut defined: BTreeMap<&str, usize> = BTreeMap::new();
    for (index, variable) in variables.iter().enumerate() {
        defined.entry(variable.name).or_insert(index);
    }

    let mut waiting_on = vec![0usize; variables.len()];
    let mut readers: Vec<Vec<usize>> = vec![Vec::new(); variables.len()];
    for (index, variable) in variables.iter().enumerate() {
        for source in dependencies(variable, &defined) {
            waiting_on[index] += 1;
            readers[source].push(index);
        }
    }

    let mut order: Vec<usize> = (0..variables.len())
        .filter(|&i| waiting_on[i] == 0)
        .collect();
    let mut head = 0;
    while head < order.len() {
        let done = order[head];
        head += 1;
        for &reader in &readers[done] {
            waiting_on[reader] -= 1;
            if waiting_on[reader] == 0 {
                order.push(reader);
            }
        }
    }

    if order.len() == variables.len() {
        Ok(order)
    } else {
        Err(EvalError::Cycle(cycle(variables, &defined, &waiting_on)))
    }
}

/// The positions of the variables that `variable` reads, in name order.
fn dependencies(variable: &Dependency<'_>, defined: &BTreeMap<&str, usize>) -> Vec<usize> {
    variable
        .reads
        .iter()
        .filter_map(|name| defined.get(name).copied())
        .collect()
}

/// One cycle, written as the chain of names that closes it.
///
/// Every variable left waiting has a dependency that is also left waiting, so
/// the walk always steps and, the graph being finite, always closes.
fn cycle(
    variables: &[Dependency<'_>],
    defined: &BTreeMap<&str, usize>,
    waiting_on: &[usize],
) -> String {
    let start = (0..variables.len())
        .find(|&i| waiting_on[i] > 0)
        .expect("an incomplete order leaves at least one variable waiting");
    let mut path = Vec::new();
    let mut seen: BTreeMap<usize, usize> = BTreeMap::new();
    let mut at = start;
    loop {
        if let Some(&first) = seen.get(&at) {
            return chain(variables, &path[first..], at);
        }
        seen.insert(at, path.len());
        path.push(at);
        at = dependencies(&variables[at], defined)
            .into_iter()
            .find(|&next| waiting_on[next] > 0)
            .expect("a waiting variable reads another waiting variable");
    }
}

fn chain(variables: &[Dependency<'_>], path: &[usize], closes: usize) -> String {
    let mut names: Vec<&str> = path.iter().map(|&i| variables[i].name).collect();
    names.push(variables[closes].name);
    names.join(" -> ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vars<'a>(pairs: &[(&'a str, &[&'a str])]) -> Vec<Dependency<'a>> {
        pairs
            .iter()
            .map(|&(name, reads)| Dependency {
                name,
                reads: reads.iter().copied().collect(),
            })
            .collect()
    }

    fn names<'a>(built: &[Dependency<'a>], order: &[usize]) -> Vec<&'a str> {
        order.iter().map(|&i| built[i].name).collect()
    }

    fn cycle_of(built: &[Dependency<'_>]) -> String {
        match evaluation_order(built) {
            Err(EvalError::Cycle(names)) => names,
            other => panic!("expected a cycle, got {other:?}"),
        }
    }

    #[test]
    fn no_variables_is_an_empty_order() {
        assert_eq!(evaluation_order(&[]), Ok(Vec::new()));
    }

    #[test]
    fn a_measurement_is_not_an_edge() {
        let built = vars(&[("raya", &["cadera"])]);
        let order = evaluation_order(&built).expect("a measurement closes no loop");
        assert_eq!(names(&built, &order), ["raya"]);
    }

    #[test]
    fn a_variable_may_reference_one_declared_later() {
        let built = vars(&[
            ("raya", &["cadera", "extension_tiro"]),
            ("extension_tiro", &["cadera"]),
        ]);
        let order = evaluation_order(&built).expect("the graph has no cycle");
        assert_eq!(names(&built, &order), ["extension_tiro", "raya"]);
    }

    #[test]
    fn independent_variables_keep_the_order_they_came_in() {
        let built = vars(&[("a", &[]), ("b", &[]), ("c", &[])]);
        let order = evaluation_order(&built).expect("the graph has no cycle");
        assert_eq!(names(&built, &order), ["a", "b", "c"]);
    }

    #[test]
    fn a_chain_is_ordered_from_its_end() {
        let built = vars(&[("a", &["b"]), ("b", &["c"]), ("c", &[])]);
        let order = evaluation_order(&built).expect("the graph has no cycle");
        assert_eq!(names(&built, &order), ["c", "b", "a"]);
    }

    #[test]
    fn a_cycle_is_reported_by_name() {
        assert_eq!(
            cycle_of(&vars(&[("a", &["b"]), ("b", &["c"]), ("c", &["a"])])),
            "a -> b -> c -> a"
        );
    }

    #[test]
    fn a_variable_that_reads_itself_is_a_cycle() {
        assert_eq!(
            cycle_of(&vars(&[("holgura", &["holgura"])])),
            "holgura -> holgura"
        );
    }

    #[test]
    fn a_variable_downstream_of_a_cycle_does_not_hide_it() {
        let built = vars(&[
            ("good", &[]),
            ("after", &["a"]),
            ("a", &["b"]),
            ("b", &["a"]),
        ]);
        assert_eq!(cycle_of(&built), "a -> b -> a");
    }
}

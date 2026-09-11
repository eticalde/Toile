/// The one thing this bake needs from a Wavefront OBJ: every vertex position
/// in file order, and the quad faces of a single named group.
pub struct Obj {
    /// Every `v` line, in file order, as xyz in the file's own units.
    pub vertices: Vec<[f64; 3]>,
    /// The 0-based vertex indices of the named group's quads, one `[a, b, c,
    /// d]` per face in the order the file winds them.
    pub group_quads: Vec<[u32; 4]>,
}

/// Parses `text`, keeping every `v` line (whatever group it falls under) and
/// the `f` lines that fall under the `g` line named `group`.
///
/// # Panics
/// If a face inside `group` is not a quad, or a `v`/`f` line does not have
/// the expected number of numbers. `base.obj`'s body group is quads-only by
/// construction; a triangle or ngon there would mean a different source file
/// was pointed at this bake.
pub fn parse(text: &str, group: &str) -> Obj {
    let mut vertices = Vec::new();
    let mut group_quads = Vec::new();
    let mut current: Option<&str> = None;
    for line in text.lines() {
        let mut words = line.split_whitespace();
        match words.next() {
            Some("v") => {
                let xyz: Vec<f64> = words.map(parse_f64).collect();
                let [x, y, z] = xyz[..] else {
                    panic!("`v` line does not have three numbers: {line}")
                };
                vertices.push([x, y, z]);
            }
            Some("g") => current = words.next(),
            Some("f") if current == Some(group) => {
                let idx: Vec<u32> = words.map(parse_index).collect();
                let [a, b, c, d] = idx[..] else {
                    panic!("face in group `{group}` is not a quad: {line}")
                };
                group_quads.push([a, b, c, d]);
            }
            _ => {}
        }
    }
    Obj {
        vertices,
        group_quads,
    }
}

fn parse_f64(tok: &str) -> f64 {
    tok.parse()
        .unwrap_or_else(|_| panic!("not a number: {tok}"))
}

/// A face token is `vertex[/texcoord[/normal]]`; only the leading, 1-based
/// vertex index matters here.
fn parse_index(tok: &str) -> u32 {
    let head = tok.split('/').next().unwrap_or(tok);
    let one_based: u32 = head
        .parse()
        .unwrap_or_else(|_| panic!("not a face index: {tok}"));
    one_based - 1
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
v 0.0 0.0 0.0
v 1.0 0.0 0.0
v 1.0 1.0 0.0
v 0.0 1.0 0.0
v 9.0 9.0 9.0
g other
f 1/1 2/2 3/3 5/5
g body
f 1/1 2/2 3/3 4/4
";

    #[test]
    fn keeps_every_vertex_and_only_the_named_groups_faces() {
        let obj = parse(SAMPLE, "body");
        assert_eq!(obj.vertices.len(), 5);
        assert_eq!(obj.group_quads, vec![[0, 1, 2, 3]]);
    }
}

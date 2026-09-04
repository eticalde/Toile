use toile_doc::{Applied, ChangeClass, PieceKey};

/// What an applied command leaves for the derivation to redo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Recompile {
    /// Nothing the solver reads has changed.
    Nothing,
    /// These pieces moved without gaining or losing a node: re-derive their
    /// rest lengths and let the drape carry on.
    Shape(Vec<PieceKey>),
    /// These pieces changed their node count: they have to be meshed again.
    Topology(Vec<PieceKey>),
}

/// Reads an applied command for what has to be recompiled.
///
/// The class comes from the command itself and the pieces from what it
/// touched, so no structural diff of the document is ever needed.
pub fn recompile(applied: &Applied) -> Recompile {
    if applied.touched.is_empty() {
        return Recompile::Nothing;
    }
    match applied.class {
        ChangeClass::Shape => Recompile::Shape(applied.touched.clone()),
        ChangeClass::Topology => Recompile::Topology(applied.touched.clone()),
        ChangeClass::Metadata | ChangeClass::Sim => Recompile::Nothing,
    }
}

#[cfg(test)]
mod tests {
    use toile_doc::{Axis, Binding, Command, Doc, MeasureSet, Piece, Point, Winding};

    use super::*;

    fn doc() -> (Doc, PieceKey, Vec<toile_doc::PointKey>) {
        let mut doc = Doc::new(MeasureSet::new("Etienne", [("cintura", 84.0)]));
        let points: Vec<toile_doc::PointKey> = [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0]]
            .into_iter()
            .map(|[x, y]| doc.points.insert(Point::at(x, y)))
            .collect();
        let piece = doc
            .pieces
            .insert(Piece::polygon("Delantero", points.clone(), Winding::Cw));
        (doc, piece, points)
    }

    fn what(doc: &mut Doc, command: Command) -> Recompile {
        recompile(&command.apply(doc).expect("the test commands are legal"))
    }

    #[test]
    fn moving_a_point_is_a_shape_edit() {
        let (mut doc, piece, points) = doc();
        let command = Command::SetBinding {
            point: points[1],
            axis: Axis::X,
            to: Binding::literal(12.0),
        };
        assert_eq!(what(&mut doc, command), Recompile::Shape(vec![piece]));
    }

    #[test]
    fn renaming_a_piece_recompiles_nothing() {
        let (mut doc, piece, _) = doc();
        let command = Command::RenamePiece {
            piece,
            to: "Trasero".to_owned(),
        };
        assert_eq!(what(&mut doc, command), Recompile::Nothing);
    }

    #[test]
    fn touched_names_only_the_pieces_that_cite_the_point() {
        let (mut doc, piece, points) = doc();
        let stray = doc.points.insert(Point::at(50.0, 50.0));
        let other = doc
            .pieces
            .insert(Piece::polygon("Trasero", points.clone(), Winding::Cw));
        let moved = Command::MovePoint {
            point: points[0],
            to: [Binding::literal(1.0), Binding::literal(1.0)],
        };
        assert_eq!(
            what(&mut doc, moved),
            Recompile::Shape(vec![piece, other]),
            "both pieces run through the point"
        );
        let elsewhere = Command::MovePoint {
            point: stray,
            to: [Binding::literal(2.0), Binding::literal(2.0)],
        };
        assert_eq!(what(&mut doc, elsewhere), Recompile::Nothing);
    }

    #[test]
    fn choosing_another_body_reshapes_every_piece() {
        let (mut doc, piece, _) = doc();
        let other = doc.mannequins.insert(MeasureSet::new("Talla 42", []));
        let command = Command::ResolveWith { mannequin: other };
        assert_eq!(what(&mut doc, command), Recompile::Shape(vec![piece]));
    }
}

//! Modelo mínimo para el Spike 2 (issue #34): una pieza con contorno de
//! puntos de control y el comando MovePoint reversible.
//!
//! Deliberadamente sin keys generacionales todavía — el spike mide el
//! pipeline incremental, no el modelo de documento completo (ese llega con
//! los índices estables del ADR §2.1 cuando el editor exista).

/// Pieza: polígono de control cerrado, en metros.
pub struct Piece {
    pub contour: Vec<[f64; 2]>,
}

pub struct Doc {
    pub pieces: Vec<Piece>,
}

/// Comando reversible. `apply` devuelve el inverso — el germen del undo.
pub enum Command {
    MovePoint {
        piece: usize,
        point: usize,
        to: [f64; 2],
    },
}

impl Command {
    pub fn apply(&self, doc: &mut Doc) -> Command {
        match *self {
            Command::MovePoint { piece, point, to } => {
                let p = &mut doc.pieces[piece].contour[point];
                let from = *p;
                *p = to;
                Command::MovePoint {
                    piece,
                    point,
                    to: from,
                }
            }
        }
    }
}

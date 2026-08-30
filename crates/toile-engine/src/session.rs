//! Sesión de alto nivel — la API que consumen los clientes (app, CLI).
//!
//! `toile-app` solo puede importar `toile-engine` (regla dura del ADR
//! §2.6): todo lo que la UI necesita pasa por aquí, sin exponer tipos del
//! solver. v0: una pieza demo drapeando sobre el avatar esfera, con
//! edición de contorno en vivo (vía A → hot-swap de rest state).

use crate::couture::{ShapePipeline, demo_bodice_contour};
use crate::sync::{self, SimHandle, Snapshot};
use std::sync::Arc;
use toile_sim::xpbd::{SdfGrid, State};

pub struct Session {
    pipeline: ShapePipeline,
    contour: Vec<[f64; 2]>,
    handle: SimHandle,
    generation: u64,
    /// Duración de la última derivación (vía A), para el HUD.
    pub last_derive_ms: f64,
}

impl Session {
    /// Pieza demo (delantero de corpiño) drapeando sobre la esfera, con la
    /// sim corriendo en su hilo a paso fijo anclado a reloj.
    pub fn demo_bodice() -> Session {
        let contour = demo_bodice_contour();
        let pipeline = ShapePipeline::build(&contour, 256, 2.0e-5);
        let n = pipeline.pos2d.len();
        let (mut cx, mut cy) = (0.0, 0.0);
        for p in &pipeline.pos2d {
            cx += p[0];
            cy += p[1];
        }
        cx /= n as f64;
        cy /= n as f64;
        let mut state = State::new(n);
        for i in 0..n {
            state.px[i] = (pipeline.pos2d[i][0] - cx) as f32;
            state.py[i] = 0.35;
            state.pz[i] = (pipeline.pos2d[i][1] - cy) as f32;
        }
        let cons = pipeline.constraints(1.0e-8);
        let sdf = SdfGrid::sphere(256, 1.4 / 255.0, [-0.7, -0.7, -0.7], [0.0, 0.0, 0.0], 0.15);
        let handle = sync::spawn(state, cons, sdf, pipeline.tris.clone(), 1.0 / 600.0, 10);
        Session {
            pipeline,
            contour,
            handle,
            generation: 0,
            last_derive_ms: 0.0,
        }
    }

    /// Contorno de control de la pieza (metros, espacio del patrón).
    pub fn contour(&self) -> &[[f64; 2]] {
        &self.contour
    }

    /// Triángulos de la malla (índices sobre las posiciones del snapshot).
    pub fn triangles(&self) -> &[u32] {
        &self.pipeline.tris
    }

    /// Último snapshot publicado por el hilo de sim (posiciones xyz
    /// intercaladas; vacío hasta el primer tick).
    pub fn snapshot(&self) -> Arc<Snapshot> {
        self.handle.snapshot()
    }

    /// Vía A en vivo: mueve un punto de control, recompila el estado de
    /// reposo y lo manda al solver residente. No resetea nada.
    pub fn move_point(&mut self, index: usize, to: [f64; 2]) {
        if index >= self.contour.len() {
            return;
        }
        self.contour[index] = to;
        let t = std::time::Instant::now();
        let rests = self.pipeline.derive(&self.contour).to_vec();
        self.last_derive_ms = t.elapsed().as_secs_f64() * 1000.0;
        self.generation += 1;
        self.handle.send_rests(self.generation, rests);
    }

    /// Radio del avatar esfera (para dibujarlo en el viewport).
    pub fn avatar_radius(&self) -> f32 {
        0.15
    }
}

impl Session {
    /// Cantidad de vértices de la malla (dimensiona buffers de render).
    pub fn n_vertices(&self) -> usize {
        self.pipeline.pos2d.len()
    }
}

//! Módulo *sync* — el hilo de simulación y su contrato de mensajes
//! (ADR §2.4). Spike 2, mitad asíncrona.
//!
//! El hilo de sim es dueño exclusivo del `State`. Comunicación solo por
//! mensajes: `RestUpdate` entra por crossbeam (latest-wins al drenar el
//! buzón entre ticks), los snapshots salen por arc-swap sin locks. Cuando
//! la tela converge y no hay mensajes, el hilo se parquea en `recv()` —
//! 0% CPU hasta la próxima edición.

use arc_swap::ArcSwap;
use crossbeam_channel::{RecvTimeoutError, Sender, TryRecvError, unbounded};
use std::sync::Arc;
use toile_sim::xpbd::{self, DistanceConstraints, SdfGrid, Seams, State};

/// Convergencia por energía cinética promedio (RMS ~2 mm/s): un vértice
/// suelto aleteando no debe impedir dormir al conjunto.
const SLEEP_ENERGY_PER_VERT: f32 = 2.0e-6;

/// Lo que el hilo de sim publica después de cada tick.
pub struct Snapshot {
    /// Última generación de rest state aplicada antes de estos substeps.
    pub generation: u64,
    /// Substeps totales ejecutados desde el spawn.
    pub substeps: u64,
    /// La sim está dormida esperando ediciones.
    pub converged: bool,
    /// Posiciones xyz intercaladas.
    pub positions: Vec<f32>,
    /// Normales por vértice xyz intercaladas (para el renderer).
    pub normals: Vec<f32>,
}

enum Msg {
    RestUpdate { generation: u64, rests: Vec<f32> },
    Stop,
}

pub struct SimHandle {
    tx: Sender<Msg>,
    snapshot: Arc<ArcSwap<Snapshot>>,
    join: Option<std::thread::JoinHandle<()>>,
}

impl SimHandle {
    /// Hot-swap del estado de reposo (vía A). No bloquea.
    pub fn send_rests(&self, generation: u64, rests: Vec<f32>) {
        let _ = self.tx.send(Msg::RestUpdate { generation, rests });
    }

    /// Último snapshot publicado. No bloquea.
    pub fn snapshot(&self) -> Arc<Snapshot> {
        self.snapshot.load_full()
    }

    pub fn stop(mut self) {
        self.shutdown();
    }

    fn shutdown(&mut self) {
        let _ = self.tx.send(Msg::Stop);
        if let Some(j) = self.join.take() {
            let _ = j.join();
        }
    }
}

impl Drop for SimHandle {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Lanza el hilo de simulación, que corre `substeps_per_tick` substeps por
/// tick y publica un snapshot al final de cada uno.
pub fn spawn(
    mut state: State,
    mut cons: DistanceConstraints,
    sdf: SdfGrid,
    tris: Vec<u32>,
    dt: f32,
    substeps_per_tick: u32,
) -> SimHandle {
    let (tx, rx) = unbounded::<Msg>();
    let snapshot = Arc::new(ArcSwap::from_pointee(Snapshot {
        generation: 0,
        substeps: 0,
        converged: false,
        positions: Vec::new(),
        normals: Vec::new(),
    }));
    let published = snapshot.clone();

    let join = std::thread::spawn(move || {
        let no_seams = Seams::default();
        let mut generation = 0u64;
        let mut substeps = 0u64;
        let mut converged = false;
        // Estado del kinetic damping (se reinicia con cada edición).
        let mut prev_e = f32::MAX;
        let mut rising = false;
        // Convergencia sostenida: ticks consecutivos quietos SIN zeroing —
        // un cero de kinetic damping deja v=0 sin estar en equilibrio.
        let mut quiet_ticks = 0u32;

        // Paso fijo anclado a reloj: cada tick representa
        // substeps_per_tick × dt de tiempo simulado y se agenda a esa
        // misma cadencia de pared — la sim no corre ni adelantada ni
        // atrasada respecto al mundo (ADR §2.4).
        let tick_period =
            std::time::Duration::from_secs_f64(f64::from(substeps_per_tick) * f64::from(dt));
        let mut next_tick = std::time::Instant::now();

        'run: loop {
            // Dormida: bloquear hasta el próximo mensaje (0% CPU).
            let mut pending = if converged {
                match rx.recv() {
                    Ok(m) => {
                        next_tick = std::time::Instant::now();
                        Some(m)
                    }
                    Err(_) => break 'run,
                }
            } else {
                // Pacing con despertar instantáneo: esperar el deadline en
                // recv_timeout — un mensaje interrumpe la espera y se
                // aplica, pero el tick mantiene su cadencia.
                let now = std::time::Instant::now();
                let mut first = None;
                if now < next_tick {
                    match rx.recv_timeout(next_tick - now) {
                        Ok(m) => first = Some(m),
                        Err(RecvTimeoutError::Timeout) => {}
                        Err(RecvTimeoutError::Disconnected) => break 'run,
                    }
                }
                first
            };
            // Drenar el buzón: el último RestUpdate gana.
            loop {
                let m = match pending.take() {
                    Some(m) => m,
                    None => match rx.try_recv() {
                        Ok(m) => m,
                        Err(TryRecvError::Empty) => break,
                        Err(TryRecvError::Disconnected) => break 'run,
                    },
                };
                match m {
                    Msg::Stop => break 'run,
                    Msg::RestUpdate {
                        generation: g,
                        rests,
                    } => {
                        cons.rest.copy_from_slice(&rests);
                        generation = g;
                        converged = false;
                        prev_e = f32::MAX;
                        rising = false;
                        quiet_ticks = 0;
                    }
                }
            }
            if converged {
                continue;
            }

            let inv_n = 1.0 / state.len() as f32;
            let mut e_avg = 0.0f32;
            for _ in 0..substeps_per_tick {
                xpbd::substep(&mut state, &cons, &no_seams, &sdf, dt);
                substeps += 1;
                let e = xpbd::kinetic_energy(&state);
                e_avg = e * inv_n;
                if e > prev_e {
                    rising = true;
                } else if rising {
                    xpbd::zero_velocities(&mut state);
                    rising = false;
                }
                prev_e = e;
            }
            if e_avg < SLEEP_ENERGY_PER_VERT {
                quiet_ticks += 1;
            } else {
                quiet_ticks = 0;
            }
            converged = quiet_ticks >= 3;
            next_tick += tick_period;
            // Si un tick tardó más que su periodo, re-anclar sin acumular deuda.
            if std::time::Instant::now() > next_tick + tick_period {
                next_tick = std::time::Instant::now();
            }

            let mut positions = Vec::with_capacity(state.len() * 3);
            for i in 0..state.len() {
                positions.push(state.px[i]);
                positions.push(state.py[i]);
                positions.push(state.pz[i]);
            }
            let mut normals = vec![0.0f32; state.len() * 3];
            xpbd::vertex_normals(&state, &tris, &mut normals);
            published.store(Arc::new(Snapshot {
                generation,
                substeps,
                converged,
                positions,
                normals,
            }));
        }
    });

    SimHandle {
        tx,
        snapshot,
        join: Some(join),
    }
}

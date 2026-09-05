# Toile

Software open source de patronaje digital: dibuja patrones en 2D, define las costuras y visualiza la prenda drapeada en 3D sobre un avatar — con vista split que refleja en 3D cada modificación del patrón 2D, sin reiniciar la simulación.

Una alternativa abierta a herramientas como CLO3D, con formato de archivo abierto y documentado.

> **Estado**: arquitectura acordada ([ADR-001](https://eticalde.github.io/Toile/architecture.html)) y **plan de validación completo** — los 6 spikes cerrados ([#33](https://github.com/eticalde/Toile/issues/33)–[#38](https://github.com/eticalde/Toile/issues/38)). Lo demostrado con números: editar el patrón re-drapea en ~7 ms con warm start; dos piezas se cosen con 10% de embebido y se editan en caliente; el estiramiento queda bajo tolerancia sartorial (0.9%); los cambios de topología transfieren el drapeado vivo en 34 ms; y todo es determinista al bit, incluso entre ARM y x86 (goldens en CI). Fase siguiente: construcción de la v1. Demos: `cargo run --release -p toile-cli -- bench --incr-async | --seams | --measure | --topo`.

## Arquitectura

La arquitectura de la v1 está decidida y documentada en el **[ADR-001](https://eticalde.github.io/Toile/architecture.html)** (fuente en [`docs/architecture.html`](docs/architecture.html)): workspace Rust de 7 crates, solver XPBD residente con warm start, derivación incremental por tipo de comando, dos hilos comunicados por mensajes, UI con egui + wgpu. El documento incluye qué se descartó y por qué, los riesgos y el plan de spikes. El plan de la pestaña Patronaje y sus decisiones viven en **[Patronaje](https://eticalde.github.io/Toile/patronaje.html)** (fuente en [`docs/patronaje.html`](docs/patronaje.html)). El de Maniquíes — personas, medidas y el cuerpo — en **[Maniquíes](https://eticalde.github.io/Toile/maniqui.html)** (fuente en [`docs/maniqui.html`](docs/maniqui.html)).

## Desarrollo

Requisitos: Rust estable (vía [rustup](https://rustup.rs)).

```sh
cargo build --workspace   # compila los 7 crates
cargo test --workspace    # tests + goldens
cargo run -p toile-cli    # CLI headless (`toile`)
./tools/verify.sh         # la puerta local: golden + fmt + estilo + clippy + tests
```

Cómo se escribe código acá está en el **[STD-001](https://eticalde.github.io/Toile/style.html)** (fuente en [`docs/style.html`](docs/style.html)): comentarios, idioma, modularización, `unsafe`, determinismo, y qué herramienta hace cumplir cada regla. CI verifica todo lo que es verificable; el resto es el checklist de PR del §10.

## Usuario objetivo

Personas que ya manejan software de patronaje (CLO, Seamly2D) o que al menos dominan el patronaje en papel. La v1 no intenta enseñar patronaje: asume que sabes qué es una pinza, un piquete y un aplomo.

## El demo que define la v1

**El vestido hola-mundo**: dibujar delantero y espalda en 2D, definir las costuras, apretar "simular" y ver la prenda vestida sobre el avatar; mover un punto en 2D y ver el 3D actualizarse sin resetear el drapeado. Si ese flujo es fluido, la v1 cumple. Todo requerimiento se mide contra este demo.

## Roadmap v1

El estado de cada tarea se sigue en el **[tablero kanban](https://github.com/users/eticalde/projects/1)** — cada ítem de esta lista es un [issue](https://github.com/eticalde/Toile/issues) etiquetado por área.

### Editor de patrones 2D

- Piezas como contornos cerrados: puntos, segmentos rectos y curvas Bézier con manijas editables
- Herramientas mínimas: dibujar, seleccionar, mover puntos, agregar/eliminar puntos, convertir recta↔curva
- Mediciones visibles: largo de cada borde en tiempo real
- Simetría: pieza espejada y media pieza con eje de doblez
- Líneas internas básicas (marcas, dobleces)
- Piquetes (notches) en bordes
- Grilla, snap, reglas, zoom/pan
- Unidades: cm, mm, pulgadas
- Duplicar, rotar y mover piezas en la mesa de trabajo

### Costuras

- Emparejar bordes (o tramos de borde) entre piezas — costura 1:1
- Indicador de dirección de costura
- Feedback visual: pares de costura identificables en ambas vistas
- Advertencia cuando los largos emparejados difieren más de una tolerancia
- Costuras 1:n (un borde contra varios segmentos) — deseable, puede ser v1.5

### Simulación y vista 3D

- Avatar fijo: una talla, pose estática (avatar propio o CC0 — cuidar licencia)
- Posicionamiento inicial de piezas alrededor del avatar
- Simulación de tela: gravedad, colisión con avatar, auto-colisión básica, costuras que unen las piezas
- Presets de tejido (algodón, denim, jersey, seda) con peso, elasticidad y rigidez de doblado
- Controles: simular / pausar / reiniciar drapeado
- Cámara orbital e iluminación básica (sin render fotorrealista)

### Vista split y sincronización — el corazón del producto

- 2D y 3D lado a lado, siempre visibles
- **Drapeado incremental (innegociable)**: editar en 2D actualiza la malla y re-drapea sin resetear la simulación
- Selección sincronizada: pieza seleccionada en 2D se resalta en 3D y viceversa

### Proyecto y archivos

- Formato de proyecto propio, abierto y documentado (JSON legible, versionado)
- Guardar / abrir proyecto
- Undo / redo en el editor 2D
- Export de patrón a SVG/PDF a escala real para imprimir
- Export 3D de la prenda drapeada (OBJ/glTF)
- Import DXF-AAMA/ASTM — deseable, probablemente v2

### No funcionales

- Simulación interactiva en hardware medio (segundos, no minutos)
- Multiplataforma
- Assets con licencias limpias (avatar CC0 o propio)

## Fuera de alcance de la v1

Tallaje/grading, márgenes de costura automáticos, tizada (marker making), texturas y estampados, avatar posable/animado, frunces y pliegues, botones/cierres/avíos, capas múltiples complejas (bolsillos), mapa de tensión/fit, colorways, colaboración. Cada uno es un proyecto en sí mismo — vendrán después.

## Licencia

[MIT](LICENSE)

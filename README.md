# Proyecto 2 - Graficas por Computadora (UVG)

Diorama estilo Minecraft renderizado 100% con raytracing en CPU, escrito en Rust.

## Estado

Repositorio inicializado. El desarrollo sigue las fases descritas en `PROGRESO.md` (se agregara en el siguiente commit de avance).

## Reglas del proyecto

- Todo el renderizado sale de lanzar rayos por pixel e intersectar la escena (sin rasterizacion, sin GPU).
- Sin librerias externas para la logica: matematica, texturas, imagenes, ruido, PRNG, paralelismo (`std::thread`) y raytracing se escriben a mano con la std de Rust.
- Unica dependencia externa permitida: la crate usada para ventana/input/framebuffer.
- Perfil release: `opt-level = 3`, `lto = "fat"`, `codegen-units = 1`, `panic = "abort"`.

## Como correr (una vez implementado)

```
cargo run --release
cargo run --release -- --render salida.png --yaw 35 --pitch 25 --dist 60
cargo run --release -- --bench
```

## Video

<!-- VIDEO AQUI -->

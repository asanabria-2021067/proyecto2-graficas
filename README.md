# El archipielago del faro

<!--
  VIDEO: pegar aca el link/embed una vez grabado (ver record.md para
  generar los cuadros con `--record` y unirlos con ffmpeg en diorama.mp4).
  Tres formatos posibles, usa el que corresponda y borra los otros:

  - YouTube con portada propia (recomendado: guarda un frame lindo del
    render como renders/portada.png y reemplaza URL_DEL_VIDEO):
    [![Video del diorama](renders/portada.png)](URL_DEL_VIDEO)

  - YouTube con la miniatura automatica (sin necesitar renders/portada.png,
    reemplaza VIDEO_ID):
    [![Video del diorama](https://img.youtube.com/vi/VIDEO_ID/0.jpg)](https://www.youtube.com/watch?v=VIDEO_ID)

  - Archivo directo (GitHub Releases, Drive, etc.) o solo un link de texto:
    [Ver video](URL_DEL_VIDEO)
-->

Diorama estilo Minecraft renderizado 100% con raytracing en CPU, escrito en
Rust desde cero (sin librerias externas para la logica: matematica,
texturas, ruido, PRNG, paralelismo y raytracing son todo codigo propio).
Cinco islas flotantes generadas proceduralmente, cada una con su propio
perfil de terreno: la isla principal (faro, lago azul-turquesa con cascada,
muelle, casita, un PUEBLO con 3 casas/torre-capilla/pozo/parcelas de
cultivo, dos islas satelite), una isla del Nether (arboles hongo carmesi/
distorsionado, formacion de blackstone con lava, fuegos, portal) y una isla
del End -- a lo sumo 6-8 bloques por encima de la principal, no flotando
muy arriba -- con una ciudad de torres de purpur, unidas por puentes de
verdad (tablero de piedra, viga y pasamanos de madera, postes con farol,
estandartes, arcos de piedra debajo).

![Vista general de las 5 islas](renders/before_after/p6_final_general_day.png)

## Requisitos

- Rust (edition 2021) con `cargo`.
- La unica dependencia externa es `raylib` (ventana, teclado/mouse y
  framebuffer). Todo lo demas -- matematica, texturas, PNG, ruido, PRNG,
  paralelismo, raytracing -- es codigo propio sobre la std.

## Como correr

```
cargo run --release
```

Abre una ventana con la escena en vivo. Tambien hay tres modos sin ventana,
utiles para revisar el trabajo o medir rendimiento:

```
# Renderiza un frame a PNG y termina (--dist hasta ~radio_principal x 9
# para que las 5 islas entren en cuadro, ver Controles)
cargo run --release -- --render salida.png --yaw 235 --pitch 30 --dist 260 --width 1280 --height 720 [--night] [--seed 7] [--no-normalmaps]

# Renderiza 30 frames desde 3 vistas fijas e imprime ms/frame promedio y FPS equivalente
cargo run --release -- --bench

# Mide el refinamiento progresivo del pase quieto pasada por pasada (sin ventana)
cargo run --release -- --bench-aa
```

## Controles

| Tecla / accion | Efecto |
|---|---|
| A / D o flechas izq/der | Rotar el diorama (yaw) |
| W / S o flechas arriba/abajo | Cambiar el angulo de camara (pitch) |
| Q / E o rueda del mouse | Zoom (alcanza para ver las 5 islas a la vez) |
| R | Activa/desactiva la auto-rotacion |
| T | Alterna dia / noche |
| N | Activa/desactiva los normal maps |
| G | Regenera las 5 islas con una nueva semilla derivada |
| 1 / 2 / 3 | Calidad baja / media / alta |
| 4 / 5 / 6 | Centra la camara (con transicion suave) en la isla principal / Nether / End |
| Esc | Salir |

El HUD (arriba a la izquierda, fuente bitmap 5x7 propia) muestra FPS, ms por
frame, modo de resolucion activo (o la pasada de refinamiento progresivo en
curso, `PASE X/Y`), resolucion interna, semilla, modo dia/noche, calidad,
normal maps on/off y el tiempo de generacion del terreno.

## Materiales

Cada material tiene su propia textura (16x16, generada por codigo si no hay
un `.bmp` en `assets/textures/`) y sus propios parametros de shading. 47 en
total, organizados por donde viven en la escena.

### Base (isla principal y satelites)

| Material | Textura | Albedo | Specular (coef/exp) | Transparencia | Reflectividad | IOR | Emision |
|---|---|---|---|---|---|---|---|
| Grass | verde con variacion, lado con franja de tierra | verde/marron | 0.05 / 8 | 0 | 0 | 1.0 | - |
| Dirt | marron con ruido | marron | 0.03 / 6 | 0 | 0 | 1.0 | - |
| Sand | tostado claro | beige | 0.05 / 10 | 0 | 0 | 1.0 | - |
| Stone bricks | ladrillos con juntas oscuras, normal map | gris | 0.15 / 24 | 0 | 0.05 | 1.0 | - |
| Oak log | vetas verticales (lado), anillos (tapa), normal map | marron | 0.05 / 8 | 0 | 0 | 1.0 | - |
| Oak planks | tablas con juntas y vetas, normal map | marron claro | 0.08 / 12 | 0 | 0 | 1.0 | - |
| Leaves | verde con alpha cutout | verde | 0.02 / 4 | 0 | 0 | 1.0 | - |
| Water | ondas suaves, absorcion Beer-Lambert | azul-turquesa | 0.6 / 90 | 0.85 | 0.30 | 1.33 | - |
| Glass | marco claro, centro casi transparente | casi blanco | 0.6 / 120 | 0.92 | 0.06 | 1.5 | - |
| Glowstone | manchas amarillas brillantes | amarillo/naranja | 0 / 1 | 0 | 0 | 1.0 | (1.0, 0.85, 0.5) x 2.5 |
| Iron block | gris metalico con borde, normal map | gris claro | 0.4 / 60 | 0 | 0.55 | 1.0 | - |
| Lamp frame | marco oscuro, centro con cutout | gris oscuro | 0.1 / 20 | 0 | 0 | 1.0 | - |

### Nether

| Material | Textura | Albedo | Specular (coef/exp) | Transparencia | Reflectividad | IOR | Emision |
|---|---|---|---|---|---|---|---|
| Netherrack | rugosa, pozos oscuros, normal map | rojo oscuro | 0.04 / 6 | 0 | 0 | 1.0 | - |
| Nether bricks | ladrillo con juntas, normal map | rojo oscuro | 0.12 / 20 | 0 | 0.03 | 1.0 | - |
| Lava | ondas, opaca | naranja intenso | 0.3 / 40 | 0 | 0 | 1.0 | (1.0, 0.45, 0.08) x 3.2 |
| Obsidian | casi negra con motas moradas | negro/morado | 0.5 / 80 | 0 | 0.35 | 1.0 | - |
| Portal | normal map propio en remolino "magico" | morado translucido | 0.4 / 30 | 0.85 | 0.05 | 1.1 | (0.55, 0.15, 0.85) x 0.6 |
| Magma | grietas via mapa de emision separado del albedo | roca oscura + grietas | 0.1 / 10 | 0 | 0 | 1.0 | solo en las grietas |
| Basalt | rayado vertical por columnas | gris azulado | 0.1 / 14 | 0 | 0 | 1.0 | - |
| Crimson stem | vetas verticales, normal map | rosa/magenta | 0.05 / 8 | 0 | 0 | 1.0 | - |
| Warped stem | vetas verticales, normal map | turquesa | 0.05 / 8 | 0 | 0 | 1.0 | - |
| Nether wart block | bultos irregulares, normal map | rojo oscuro | 0.02 / 4 | 0 | 0 | 1.0 | - |
| Warped wart block | bultos irregulares, normal map | turquesa oscuro | 0.02 / 4 | 0 | 0 | 1.0 | - |
| Shroomlight | manchas calidas | naranja/amarillo | 0.1 / 10 | 0 | 0 | 1.0 | (1.0, 0.6, 0.25) x 2.2 |
| Crimson nylium | uniforme (cara de abajo: netherrack) | rojo/magenta | 0.04 / 6 | 0 | 0 | 1.0 | - |
| Warped nylium | uniforme (cara de abajo: netherrack) | turquesa | 0.04 / 6 | 0 | 0 | 1.0 | - |
| Blackstone | motas claras dispersas, normal map | gris muy oscuro | 0.15 / 22 | 0 | 0.04 | 1.0 | - |
| Fire | silueta de llama recortada (alpha cutout) | naranja/amarillo | 0 / 1 | 0 | 0 | 1.0 | (1.0, 0.55, 0.15) x 2.0 |
| Soul fire | silueta de llama recortada (alpha cutout) | celeste/azul | 0 / 1 | 0 | 0 | 1.0 | (0.3, 0.75, 1.0) x 2.0 |
| Soul sand | punteado oscuro | gris violaceo | 0.02 / 4 | 0 | 0 | 1.0 | - |

### End

| Material | Textura | Albedo | Specular (coef/exp) | Transparencia | Reflectividad | IOR | Emision |
|---|---|---|---|---|---|---|---|
| End stone | motas oscuras dispersas, normal map | amarillo verdoso palido | 0.06 / 10 | 0 | 0 | 1.0 | - |
| End stone bricks | ladrillo claro, normal map | amarillo palido | 0.1 / 16 | 0 | 0 | 1.0 | - |
| Purpur | cuadricula, normal map | morado | 0.25 / 30 | 0 | 0 | 1.0 | - |
| Purpur pillar | rayas en columna, normal map | morado | 0.28 / 32 | 0 | 0 | 1.0 | - |
| End crystal | gradiente radial | rosa/morado | 0.7 / 100 | 0.6 | 0.5 | 1.6 | (0.9, 0.55, 1.0) x 1.8 |
| End rod | bloque entero, lampara | blanco | 0.3 / 40 | 0 | 0 | 1.0 | (0.9, 0.95, 1.0) x 2.0 |
| Chorus (flor) | planta con alpha cutout | morado claro | 0.05 / 6 | 0 | 0 | 1.0 | - |
| Chorus plant (tallo) | solido con nudos | morado oscuro | 0.04 / 6 | 0 | 0 | 1.0 | - |
| Magenta glass | marco claro, centro casi transparente | magenta | 0.6 / 120 | 0.85 | 0.08 | 1.5 | - |

### Pueblo (isla principal)

| Material | Textura | Albedo | Specular (coef/exp) | Transparencia | Reflectividad | IOR | Emision |
|---|---|---|---|---|---|---|---|
| Cobblestone | piedras irregulares con mortero, normal map | gris | 0.08 / 12 | 0 | 0.02 | 1.0 | - |
| Stone (liso) | uniforme, normal map suave | gris claro | 0.12 / 18 | 0 | 0.04 | 1.0 | - |
| Farmland | tierra removida con surcos | marron oscuro | 0.02 / 4 | 0 | 0 | 1.0 | - |
| Dirt path | grisaceo apisonado, a proposito distinto del marron de oak_planks | gris-tostado | 0.03 / 6 | 0 | 0 | 1.0 | - |
| Crops | tallos con alpha cutout, puntas doradas | verde/dorado | 0.03 / 6 | 0 | 0 | 1.0 | - |
| Lantern | marco oscuro, centro calido opaco | negro/amarillo calido | 0.15 / 20 | 0 | 0 | 1.0 | (1.0, 0.72, 0.38) x 1.9 |

La cerca de las parcelas reutiliza `oak_log` en postes delgados en vez de
un material nuevo.

### Puentes

| Material | Textura | Albedo | Specular (coef/exp) | Transparencia | Reflectividad | IOR | Emision |
|---|---|---|---|---|---|---|---|
| Banner | tela colgando con alpha cutout, franja dorada arriba, puntas en V abajo | rojo/dorado | 0.02 / 4 | 0 | 0 | 1.0 | - |

Estandarte que cuelga de la mitad de los postes de cada puente (ver
"Puentes de verdad" mas abajo). El tablero, la viga/pasamanos y los arcos
reutilizan materiales ya existentes (piedra clara `stone` + `oak_log` en
los satelites, `nether_bricks` + `blackstone` en el Nether, `end_stone_bricks`
+ `purpur` en el End), con `lantern`/`glowstone`/`end_rod` como farol segun
el destino.

Normal map real (derivado por Sobel de la propia textura, o cargado desde
`assets/textures/<nombre>_n.bmp` si existe) en: `stone_bricks`,
`oak_planks`, `oak_log`, `iron_block`, `netherrack`, `nether_bricks`,
`crimson_stem`, `warped_stem`, `nether_wart_block`, `warped_wart_block`,
`blackstone`, `end_stone`, `end_stone_bricks`, `purpur`, `purpur_pillar`,
`cobblestone`, `stone`.
El `portal` tiene un normal map propio generado aparte (no derivado del
albedo): un remolino via atan2/seno/coseno, para que la refraccion se vea
distorsionada en vez de plana.

## Tecnicas usadas

- **DDA multi-isla (Amanatides & Woo)**: cada isla (y cada puente, y cada
  mini-isla de descanso) es su propia grilla de voxeles con un offset
  entero que la ubica en el mundo (`src/islands.rs`). Un rayo primero se
  prueba contra el AABB de cada isla (sin asignar memoria, ordenado por t
  de entrada) y solo corre el DDA voxel a voxel dentro de las que
  realmente puede llegar a tocar. Sombras, reflejos y refracciones cruzan
  islas sin logica especial.
- **Puentes reutilizables, un solo esqueleto**: `BridgeStyle` +
  `build_bridge_span` arman cada puente como su propia mini-isla; tablero
  recto de 3 de ancho, viga de borde y pasamanos horizontal continuos,
  postes de 2 de alto (siempre arriba del tablero, nunca cuelgan) cada 5
  pasos con un farol encima, estandartes colgando de la mitad de los
  postes, y 2-3 arcos de piedra cortos y anchos debajo (patas + dintel,
  maximo 3 bloques de caida). Solo cambian los materiales segun el destino
  (`BridgeStyle`). Si el hueco entre dos islas es muy grande, se insertan
  mini-islas de descanso en el medio.
- **Fresnel (Schlick) y Snell**: la reflexion y la refraccion se calculan
  juntas -- Snell da la direccion refractada con el IOR del material,
  Fresnel-Schlick reparte cuanta energia va a reflexion y cuanta a
  refraccion segun el angulo de incidencia; la reflexion interna total
  redirige toda la energia a reflexion.
- **Beer-Lambert por medio, no por voxel**: la luz que atraviesa un medio
  transparente (agua, portal, vidrio) se atenua por canal segun la
  distancia REAL recorrida dentro del volumen completo -- tanto para el
  rayo de vista como para el de sombra, que rastrea el medio actual igual
  que los rayos primarios en vez de tratar cada voxel interno como una
  superficie separada.
- **Normal maps con TBN**: cada cara del cubo tiene una tangente/bitangente
  fija coherente con sus UV; el normal map (cargado, derivado por Sobel de
  la textura, o generado aparte como el remolino del portal) perturba la
  normal antes de calcular difusa, especular, reflexion y refraccion.
- **Emision por mapa**: ademas de la emision plana por material (glowstone,
  lava, end_rod, end_crystal, shroomlight, fire, soul_fire), el magma usa
  un mapa de emision separado del albedo (mismo patron de grietas, pixel a
  pixel) para que solo las grietas brillen, no todo el bloque.
- **Cubemap continuo en 3D**: el skybox es un cubo de 6 caras de 256x256;
  las nubes se generan con fBm 3D muestreado sobre la direccion normalizada
  (no 2D por cara), asi son continuas entre caras sin costuras ni
  estiramiento, con una banda de densidad cerca del horizonte que se
  desvanece hacia el cenit y hacia abajo. Campo de estrellas por hash y
  discos de sol/luna con glow de noche.
- **Perlin / fBm, tres perfiles distintos**: ruido Perlin 2D/3D con
  permutacion generada desde una semilla (PCG32 propio). La isla principal
  usa fBm suave normal; el Nether pliega el fBm sobre si mismo ("ridged")
  para picos filosos y grietas; el End usa poca amplitud redondeada a
  escalones de 2 bloques (terrazas) y casi nada de deformacion de borde.
- **Paralelismo por tiles**: el framebuffer se reparte en tiles de 16x16
  tomados de una cola atomica (`AtomicUsize`) por `std::thread::scope`, sin
  `Mutex` en el camino caliente.
- **Refinamiento progresivo y adaptativo**: la ventana siempre corre a
  resolucion real (1:1, minimo 1280x720). Mientras la camara se mueve,
  renderiza a una fraccion segun calidad y escala con nearest-neighbor. Al
  soltar, el pase quieto NO bloquea de una vez: la pasada 0 tira 1
  muestra/pixel sobre todo el frame (algo nitido de inmediato), calcula
  una mascara de contraste, y las pasadas siguientes (una por frame, sin
  bloquear los controles) solo refinan los pixeles de alto contraste hasta
  completar la calidad elegida. Se cancela si la camara se mueve.

## Rendimiento

Medido con `cargo run --release -- --bench` en esta maquina (20 hilos
detectados), sobre la escena completa (principal + 2 satelite + Nether +
End + todos sus puentes/caminos y mini-islas):

| Resolucion | ms/frame | FPS equivalente |
|---|---|---|
| 640x360 (resolucion del pase "en movimiento" a calidad media) | 32.6 | ~30.7 |
| 1280x720 | 128.4 | ~7.8 |

Rotando a calidad media la ventana corre a resolucion reducida (~30.7 FPS
equivalente, fluido); el pase de resolucion completa es ahora PROGRESIVO
(ver "Refinamiento progresivo" arriba), asi que nunca bloquea de una sola
vez aunque tarde varios cientos de ms en total. Rehacer los 4 puentes con
postes/farol cada 5 pasos (antes cada 6, alternando de lado -- la mitad de
luces) bajo el rendimiento en serio (~22 a ~16 FPS en calidad media). Se
bajo el radio de cada luz (`lights.rs`) y el limite de luces evaluadas por
punto (`shading.rs`, `MAX_LIGHTS_PER_POINT` 4 -> 3): ~-35% en 640x360 y
1280x720, bien por encima del piso de 20-22 FPS. Detalle completo, incluido
el desglose pasada por pasada del refinamiento progresivo, en
`BENCHMARK.md`.

## Galeria

| De dia | De noche |
|---|---|
| ![Vista general de dia](renders/before_after/p6_final_general_day.png) | ![Vista general de noche](renders/before_after/p6_final_general_night.png) |
| ![Puentes de verdad y el pueblo](renders/before_after/p6_final_bridges.png) | ![Nether de noche](renders/before_after/p6_final_nether_night.png) |
| ![Lago azul-turquesa con orilla](renders/before_after/p6_final_lake.png) | ![Ciudad de torres del End](renders/before_after/p6_final_end_city.png) |
| ![El pueblo de dia](renders/before_after/part5_village_day.png) | ![El pueblo de noche, faroles encendidos](renders/before_after/part5_village_night.png) |

## Guion sugerido para el video

1. Arrancar con la vista general (zoom alejado, las 5 islas -- principal,
   sus 2 satelites, Nether y End -- en un solo cuadro, puentes bien
   visibles entre todas) unos segundos quieto.
2. Activar auto-rotacion (`R`) para dar una vuelta completa al archipielago.
3. Detener la rotacion. Cruzar uno de los puentes de piedra hacia el
   monolito de hierro (reflejo), volando pegado al tablero para mostrar los
   postes con farol, la viga y el pasamanos de madera, los estandartes
   colgando y los arcos de soporte debajo.
4. Tecla `5` para centrar suavemente la camara en el Nether, alternar a
   noche (`T`) y hacer zoom (`Q`/`E`) hacia los arboles hongo (carmesi y
   distorsionado), la formacion de blackstone con la cascada de lava, los
   fuegos y el portal -- mostrar lo imponente que se ve de noche.
5. Tecla `6` para centrar en el End: zoom hacia la ciudad de torres (torre
   central, escaleras diagonales, las dos torres secundarias con
   ventanas de magenta_glass), el barco de purpur y los bosquecitos de
   chorus.
6. Tecla `4` para volver a la principal: recorrer el pueblo (casas, la
   torre-capilla, el pozo, las parcelas con canal de agua y crops, los
   caminos), acercarse al faro (glowstone a traves del vidrio) y al lago
   (agua azul-turquesa con reflejo del cielo, orilla de arena visible,
   cascada por el borde).
7. Alternar normal maps (`N`) de cerca sobre nether_bricks o stone_bricks
   para mostrar la diferencia con luz rasante.
8. Regenerar el archipielago (`G`) un par de veces para mostrar que las 5
   islas, sus puentes, el pueblo y todas sus estructuras se reconstruyen
   con cualquier semilla.

El video grabado con `--record` (ver `record.md`) sigue este mismo recorrido
como guion fijo de camara (`src/record.rs::timeline`, 83s): vista general,
un tramo dedicado al pueblo (~6s, entre la vista general y el acercamiento
al faro), faro/lago, un puente de cerca (~4s, postes/farol/arcos/
estandartes) camino al monolito, normal maps on/off, noche, Nether de
cerca, End de cerca, y regeneracion con 2 semillas nuevas antes de la toma
final.

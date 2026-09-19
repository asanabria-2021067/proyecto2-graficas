//! Guion de la camara para `--record`: una lista fija de keyframes (tiempo en
//! segundos, yaw/pitch/distancia relativa al radio de la isla principal,
//! hacia que isla mira, y el estado dia/noche + normal maps como un blend
//! [0,1] en vez de un bool) que `sample_timeline` interpola con ease in/out
//! (smoothstep) entre el par de keyframes que rodea el instante pedido.
//! Separado de `main.rs` porque es puro dato+matematica, sin nada de
//! renderizado ni de `WorldData`.
//!
//! Regla de armado (importante para no repetir los saltos de camara de la
//! version anterior): dos keyframes consecutivos SIEMPRE deben describir un
//! movimiento continuo real (el que se ve en pantalla durante ese tramo).
//! Cuando un tramo pide "camara quieta" (la parte `e`, normal maps), sus
//! keyframes deben tener yaw/pitch/dist/centro IDENTICOS entre si -- el
//! unico cambio ahi es el blend de normal maps. Un cambio de semilla, en
//! cambio, SI es un corte intencional (no interpola: ver `seed_at`).

use crate::math::Vec3;

#[derive(Clone, Copy, PartialEq)]
pub enum CenterTarget {
    Main,
    Nether,
    End,
    Monolith,
    Village,
}

/// Puntos de mundo donde puede centrarse la orbita, resueltos por el llamador
/// (parte de `WorldData`, que vive en `main.rs`) antes de samplear.
#[derive(Clone, Copy)]
pub struct Centers {
    pub main: Vec3,
    pub nether: Vec3,
    pub end: Vec3,
    pub monolith: Vec3,
    pub village: Vec3,
}

pub struct Keyframe {
    pub t: f32,
    pub yaw_deg: f32,
    pub pitch_deg: f32,
    /// Multiplicado por el radio de la isla principal al samplear, asi la
    /// distancia de camara no depende de valores absolutos por isla.
    pub dist_scale: f32,
    pub center: CenterTarget,
    pub night: f32,      // 0.0 = dia, 1.0 = noche
    pub normalmaps: f32, // 0.0 = off, 1.0 = on
    pub seed_offset: u32,
    /// Nombre del tramo que EMPIEZA en este keyframe (se muestra durante
    /// `[este.t, siguiente.t)`), para el HUD y para `--dump-timeline`.
    pub segment: &'static str,
    pub caption: &'static str,
}

pub struct Sample {
    pub yaw_deg: f32,
    pub pitch_deg: f32,
    pub dist: f32,
    pub center: Vec3,
    pub night_blend: f32,
    pub normalmaps_blend: f32,
    pub segment: &'static str,
    pub caption: &'static str,
}

#[inline]
fn ease(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[inline]
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn resolve_center(centers: Centers, target: CenterTarget) -> Vec3 {
    match target {
        CenterTarget::Main => centers.main,
        CenterTarget::Nether => centers.nether,
        CenterTarget::End => centers.end,
        CenterTarget::Monolith => centers.monolith,
        CenterTarget::Village => centers.village,
    }
}

const CAP_GENERAL: &str = "VISTA GENERAL - DIORAMA DE 5 ISLAS";
const CAP_MAIN: &str = "ISLA PRINCIPAL: FARO Y CASITA";
const CAP_VILLAGE: &str = "PUEBLO: CASAS, TORRE, PARCELAS Y POZO";
const CAP_LAKE: &str = "LAGO: REFRACCION IOR 1.33 Y REFLEJO FRESNEL";
const CAP_MONOLITH: &str = "REFLEJO EN EL MONOLITO - IRON BLOCK Y CIELO";
const CAP_NM_OFF: &str = "NORMAL MAPS OFF - LUZ RASANTE EN STONE BRICKS";
const CAP_NM_FADE: &str = "NORMAL MAPS: FUNDIDO OFF A ON";
const CAP_NM_ON: &str = "NORMAL MAPS ON - RELIEVE CON SOBEL";
const CAP_NIGHT: &str = "NOCHE: EMISIVOS Y LINTERNAS DE PUENTES Y FARO";
const CAP_NETHER: &str = "NETHER: LAVA FUEGOS PORTAL Y ARBOLES HONGO";
const CAP_END: &str = "END: CIUDAD DE TORRES END CRYSTAL Y BARCO";
const CAP_SEED_B: &str = "REGENERACION - SEMILLA B";
const CAP_SEED_C: &str = "REGENERACION - SEMILLA C";
const CAP_FINAL: &str = "DIORAMA PROCEDURAL COMPLETO";

/// Guion de 79s, un keyframe por limite de tramo (18 en total, ver
/// comentarios inline con los segundos exactos). Cada tramo mueve como
/// maximo un par de cosas a la vez (camara, o dia/noche, o normal maps, o
/// semilla) para que quede claro en pantalla que esta cambiando; la unica
/// excepcion son las traslaciones largas entre islas/el pueblo (a2, f, g1,
/// h1), donde camara + centro (+ a veces dia/noche) cambian juntos porque
/// asi se ve como un solo movimiento de camara continuo, no como una serie
/// de cortes.
///
/// Recalculado despues de mover/agrandar el End y el Nether (ver
/// `structures.rs`): el End ahora flota apenas mas arriba que la principal
/// (antes offset.y +32, ahora +9) y mas cerca (gap 10 en vez de 16), y tanto
/// el Nether (radio 11->15) como el End (radio 16->22) crecieron. Los tramos
/// g2/h1/h2/i1 (los que miran de cerca esas dos islas) necesitan mas
/// distancia y menos pitch que antes para no terminar con la camara metida
/// en el terreno -- afinado a mano con `--render --center nether|end` a
/// varias combinaciones hasta que el encuadre mostraba la isla entera.
pub fn timeline(seed_a: u32, seed_b: u32, seed_c: u32) -> Vec<Keyframe> {
    vec![
        // a) 0-8s: vista general de dia, rotando.
        Keyframe { t: 0.0, yaw_deg: 0.0, pitch_deg: 22.0, dist_scale: 6.5, center: CenterTarget::Main, night: 0.0, normalmaps: 1.0, seed_offset: seed_a, segment: "a: general", caption: CAP_GENERAL },
        // a2) 8-14s (6s, NUEVO): recorrido del pueblo -- casas, parcelas,
        // pozo y caminos -- entre la vista general y el acercamiento al faro.
        Keyframe { t: 8.0, yaw_deg: 100.0, pitch_deg: 25.0, dist_scale: 0.9, center: CenterTarget::Village, night: 0.0, normalmaps: 1.0, seed_offset: seed_a, segment: "a2: pueblo", caption: CAP_VILLAGE },
        // b) 14-21s (7s): zoom a la isla principal (faro/casita), llegando al lago.
        Keyframe { t: 14.0, yaw_deg: 70.0, pitch_deg: 20.0, dist_scale: 6.5, center: CenterTarget::Main, night: 0.0, normalmaps: 1.0, seed_offset: seed_a, segment: "b: principal", caption: CAP_MAIN },
        // c) 21-27s (6s): orbita lenta de cerca sobre el lago.
        Keyframe { t: 21.0, yaw_deg: 110.0, pitch_deg: 14.0, dist_scale: 1.0, center: CenterTarget::Main, night: 0.0, normalmaps: 1.0, seed_offset: seed_a, segment: "c: lago", caption: CAP_LAKE },
        // d) 27-31s (4s): vuelo hasta el monolito y de vuelta hacia el faro.
        Keyframe { t: 27.0, yaw_deg: 150.0, pitch_deg: 14.0, dist_scale: 1.0, center: CenterTarget::Main, night: 0.0, normalmaps: 1.0, seed_offset: seed_a, segment: "d: monolito (ida)", caption: CAP_MONOLITH },
        Keyframe { t: 29.0, yaw_deg: 175.0, pitch_deg: 16.0, dist_scale: 0.55, center: CenterTarget::Monolith, night: 0.0, normalmaps: 1.0, seed_offset: seed_a, segment: "d: monolito (vuelta)", caption: CAP_MONOLITH },
        // e) 31-38s: camara TOTALMENTE QUIETA (mismo yaw/pitch/dist/centro en
        // los 4 keyframes que siguen), solo cambia el blend de normal maps.
        Keyframe { t: 31.0, yaw_deg: 200.0, pitch_deg: 10.0, dist_scale: 0.85, center: CenterTarget::Main, night: 0.0, normalmaps: 0.0, seed_offset: seed_a, segment: "e: nm off (quieto)", caption: CAP_NM_OFF },
        Keyframe { t: 34.0, yaw_deg: 200.0, pitch_deg: 10.0, dist_scale: 0.85, center: CenterTarget::Main, night: 0.0, normalmaps: 0.0, seed_offset: seed_a, segment: "e: nm fundido (quieto)", caption: CAP_NM_FADE },
        Keyframe { t: 35.0, yaw_deg: 200.0, pitch_deg: 10.0, dist_scale: 0.85, center: CenterTarget::Main, night: 0.0, normalmaps: 1.0, seed_offset: seed_a, segment: "e: nm on (quieto)", caption: CAP_NM_ON },
        // f) 38-44s (6s): noche, vista general (se aleja y cae la noche).
        Keyframe { t: 38.0, yaw_deg: 200.0, pitch_deg: 10.0, dist_scale: 0.85, center: CenterTarget::Main, night: 0.0, normalmaps: 1.0, seed_offset: seed_a, segment: "f: a noche general", caption: CAP_NIGHT },
        // g) 44-54s (10s): Nether -- g1 acercandose (4s), g2 de cerca (6s).
        // Nota sobre `dist_scale` de aca en adelante: se multiplica por el
        // radio de la isla PRINCIPAL (42), no por el de la isla que mira la
        // camara -- Nether (radio 15, a ~65 del centro principal) y End
        // (radio 22, a ~74) son mas chicas Y ademas tienen relieve marcado
        // (el ridged noise del Nether sube/baja ~10 bloques sobre su propia
        // base, las torres del End suben otros ~20). Un pitch bajo (el que
        // usa el lago, isla mucho mas playa) deja al ojo de la camara CASI A
        // LA MISMA altura que esos picos/torres -- de ahi el cuadro con la
        // camara metida dentro del terreno. Pitch ~32-35 con distancia
        // bastante mayor que el radio de esa isla le da altura y espacio de
        // sobra sin alejarse tanto como para perder el detalle.
        Keyframe { t: 44.0, yaw_deg: 230.0, pitch_deg: 18.0, dist_scale: 6.0, center: CenterTarget::Main, night: 1.0, normalmaps: 1.0, seed_offset: seed_a, segment: "g1: hacia el nether", caption: CAP_NETHER },
        Keyframe { t: 48.0, yaw_deg: 250.0, pitch_deg: 32.0, dist_scale: 1.0, center: CenterTarget::Nether, night: 1.0, normalmaps: 1.0, seed_offset: seed_a, segment: "g2: nether de cerca", caption: CAP_NETHER },
        // h) 54-64s (10s): End -- h1 acercandose (4s), h2 de cerca (6s).
        Keyframe { t: 54.0, yaw_deg: 290.0, pitch_deg: 32.0, dist_scale: 1.0, center: CenterTarget::Nether, night: 1.0, normalmaps: 1.0, seed_offset: seed_a, segment: "h1: hacia el end", caption: CAP_END },
        Keyframe { t: 58.0, yaw_deg: 320.0, pitch_deg: 35.0, dist_scale: 1.1, center: CenterTarget::End, night: 1.0, normalmaps: 1.0, seed_offset: seed_a, segment: "h2: end de cerca", caption: CAP_END },
        // i) 64-74s: regeneracion -- 5s con la semilla B, 5s con la semilla C
        // (el corte de semilla es instantaneo, la camara sigue moviendose
        // parejo de la vista del End a la vista general de ambos lados del corte).
        Keyframe { t: 64.0, yaw_deg: 350.0, pitch_deg: 35.0, dist_scale: 1.1, center: CenterTarget::End, night: 1.0, normalmaps: 1.0, seed_offset: seed_b, segment: "i1: semilla B", caption: CAP_SEED_B },
        Keyframe { t: 69.0, yaw_deg: 380.0, pitch_deg: 22.0, dist_scale: 6.5, center: CenterTarget::Main, night: 0.0, normalmaps: 1.0, seed_offset: seed_c, segment: "i2: semilla C", caption: CAP_SEED_C },
        // j) 74-79s (5s): toma final alejandose.
        Keyframe { t: 74.0, yaw_deg: 420.0, pitch_deg: 24.0, dist_scale: 7.0, center: CenterTarget::Main, night: 0.0, normalmaps: 1.0, seed_offset: seed_c, segment: "j: final", caption: CAP_FINAL },
        Keyframe { t: 79.0, yaw_deg: 450.0, pitch_deg: 26.0, dist_scale: 8.5, center: CenterTarget::Main, night: 0.0, normalmaps: 1.0, seed_offset: seed_c, segment: "j: final", caption: CAP_FINAL },
    ]
}

pub fn total_duration(kfs: &[Keyframe]) -> f32 {
    kfs.last().map(|k| k.t).unwrap_or(0.0)
}

fn active_index(kfs: &[Keyframe], t: f32) -> usize {
    let mut idx = 0;
    for (i, k) in kfs.iter().enumerate() {
        if k.t <= t {
            idx = i;
        } else {
            break;
        }
    }
    idx
}

/// Que semilla esta activa en el instante `t`, sin necesitar los centros de
/// las islas -- se consulta ANTES de reconstruir `WorldData` para saber si
/// hace falta regenerar, y recien despues se llama a `sample_timeline` (que
/// si necesita los centros, ya actualizados) para el resto del estado.
pub fn seed_at(kfs: &[Keyframe], t: f32) -> u32 {
    kfs[active_index(kfs, t)].seed_offset
}

/// Samplea el guion en el instante `t`: interpola yaw/pitch/dist/centro/
/// night/normalmaps con ease in/out entre el keyframe activo y el
/// siguiente; `seed_offset`/`segment`/`caption` se toman del keyframe activo
/// (el mas reciente con `t_kf <= t`), sin interpolar -- un cambio de semilla
/// es un corte, no una transicion continua, y un tramo es una etiqueta, no
/// algo que tenga sentido mezclar con el siguiente.
pub fn sample_timeline(kfs: &[Keyframe], centers: Centers, t: f32) -> Sample {
    let last = kfs.len() - 1;
    let idx = active_index(kfs, t);
    let a = &kfs[idx];
    if idx == last {
        return Sample {
            yaw_deg: a.yaw_deg,
            pitch_deg: a.pitch_deg,
            dist: a.dist_scale,
            center: resolve_center(centers, a.center),
            night_blend: a.night,
            normalmaps_blend: a.normalmaps,
            segment: a.segment,
            caption: a.caption,
        };
    }
    let b = &kfs[idx + 1];
    let f = ease((t - a.t) / (b.t - a.t));
    Sample {
        yaw_deg: lerp(a.yaw_deg, b.yaw_deg, f),
        pitch_deg: lerp(a.pitch_deg, b.pitch_deg, f),
        dist: lerp(a.dist_scale, b.dist_scale, f),
        center: resolve_center(centers, a.center).lerp(resolve_center(centers, b.center), f),
        night_blend: lerp(a.night, b.night, f),
        normalmaps_blend: lerp(a.normalmaps, b.normalmaps, f),
        segment: a.segment,
        caption: a.caption,
    }
}

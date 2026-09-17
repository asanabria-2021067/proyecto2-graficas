//! Guion de la camara para `--record`: una lista fija de keyframes (tiempo en
//! segundos, yaw/pitch/distancia relativa al radio de la isla principal,
//! hacia que isla mira, y el estado dia/noche + normal maps como un blend
//! [0,1] en vez de un bool) que `sample_timeline` interpola con ease in/out
//! (smoothstep) entre el par de keyframes que rodea el instante pedido.
//! Separado de `main.rs` porque es puro dato+matematica, sin nada de
//! renderizado ni de `WorldData`.

use crate::math::Vec3;

#[derive(Clone, Copy, PartialEq)]
pub enum CenterTarget {
    Main,
    Nether,
    End,
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
    pub caption: &'static str,
}

pub struct Sample {
    pub yaw_deg: f32,
    pub pitch_deg: f32,
    pub dist: f32,
    pub center: Vec3,
    pub night_blend: f32,
    pub normalmaps_blend: f32,
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

fn resolve_center(centers: (Vec3, Vec3, Vec3), target: CenterTarget) -> Vec3 {
    match target {
        CenterTarget::Main => centers.0,
        CenterTarget::Nether => centers.1,
        CenterTarget::End => centers.2,
    }
}

/// Guion de ~74s: (a) vista general con giro completo, (b) zoom a la isla
/// principal (faro/lago/casita), (c) normal maps off->on con luz rasante,
/// (d) transicion a noche en vista general, (e) zoom al Nether de noche,
/// (f) zoom al End, (g) corte a 2 semillas nuevas en vista general, (h) vista
/// general final alejandose. `seed_offset` se sigue como funcion escalon
/// (toma el valor del keyframe IZQUIERDO del tramo activo) porque cambiar de
/// semilla implica regenerar todo el mundo -- no algo que tenga sentido
/// interpolar; yaw/pitch/dist/centro/night/normalmaps si son continuos.
pub fn timeline(seed_a: u32, seed_b: u32, seed_c: u32) -> Vec<Keyframe> {
    vec![
        Keyframe { t: 0.0, yaw_deg: 0.0, pitch_deg: 22.0, dist_scale: 6.5, center: CenterTarget::Main, night: 0.0, normalmaps: 1.0, seed_offset: seed_a, caption: "VISTA GENERAL - DIORAMA DE 5 ISLAS" },
        Keyframe { t: 13.0, yaw_deg: 360.0, pitch_deg: 22.0, dist_scale: 6.5, center: CenterTarget::Main, night: 0.0, normalmaps: 1.0, seed_offset: seed_a, caption: "VISTA GENERAL - DIORAMA DE 5 ISLAS" },
        Keyframe { t: 19.0, yaw_deg: 395.0, pitch_deg: 18.0, dist_scale: 1.9, center: CenterTarget::Main, night: 0.0, normalmaps: 1.0, seed_offset: seed_a, caption: "ISLA PRINCIPAL: FARO Y CASITA" },
        Keyframe { t: 26.0, yaw_deg: 410.0, pitch_deg: 14.0, dist_scale: 1.1, center: CenterTarget::Main, night: 0.0, normalmaps: 1.0, seed_offset: seed_a, caption: "LAGO: REFRACCION IOR 1.33 Y REFLEJO FRESNEL" },
        Keyframe { t: 28.0, yaw_deg: 410.0, pitch_deg: 12.0, dist_scale: 0.9, center: CenterTarget::Main, night: 0.0, normalmaps: 0.0, seed_offset: seed_a, caption: "NORMAL MAPS OFF - LUZ RASANTE EN STONE BRICKS" },
        Keyframe { t: 32.0, yaw_deg: 415.0, pitch_deg: 12.0, dist_scale: 0.9, center: CenterTarget::Main, night: 0.0, normalmaps: 1.0, seed_offset: seed_a, caption: "NORMAL MAPS ON - RELIEVE CON SOBEL" },
        Keyframe { t: 40.0, yaw_deg: 430.0, pitch_deg: 20.0, dist_scale: 6.5, center: CenterTarget::Main, night: 1.0, normalmaps: 1.0, seed_offset: seed_a, caption: "NOCHE: EMISIVOS Y LINTERNAS DE PUENTES Y FARO" },
        Keyframe { t: 50.0, yaw_deg: 460.0, pitch_deg: 18.0, dist_scale: 1.6, center: CenterTarget::Nether, night: 1.0, normalmaps: 1.0, seed_offset: seed_a, caption: "NETHER: LAVA FUEGOS PORTAL Y ARBOLES HONGO" },
        Keyframe { t: 60.0, yaw_deg: 490.0, pitch_deg: 18.0, dist_scale: 1.6, center: CenterTarget::End, night: 1.0, normalmaps: 1.0, seed_offset: seed_a, caption: "END: CIUDAD DE TORRES END CRYSTAL Y BARCO" },
        Keyframe { t: 64.0, yaw_deg: 500.0, pitch_deg: 22.0, dist_scale: 6.5, center: CenterTarget::Main, night: 0.0, normalmaps: 1.0, seed_offset: seed_b, caption: "REGENERACION - SEMILLA B" },
        Keyframe { t: 68.0, yaw_deg: 500.0, pitch_deg: 22.0, dist_scale: 6.5, center: CenterTarget::Main, night: 0.0, normalmaps: 1.0, seed_offset: seed_c, caption: "REGENERACION - SEMILLA C" },
        Keyframe { t: 74.0, yaw_deg: 540.0, pitch_deg: 26.0, dist_scale: 8.5, center: CenterTarget::Main, night: 0.0, normalmaps: 1.0, seed_offset: seed_c, caption: "DIORAMA PROCEDURAL COMPLETO" },
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
/// siguiente; `seed_offset`/`caption` se toman del keyframe activo (el mas
/// reciente con `t_kf <= t`), sin interpolar -- un cambio de semilla es un
/// corte, no una transicion continua.
pub fn sample_timeline(kfs: &[Keyframe], centers: (Vec3, Vec3, Vec3), t: f32) -> Sample {
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
        caption: a.caption,
    }
}

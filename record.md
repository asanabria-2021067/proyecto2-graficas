# Grabacion del video

`--record` renderiza el guion de camara de `src/record.rs` cuadro por cuadro
en maxima calidad (SSAA completo, sin el refinamiento progresivo del modo
ventana) y los guarda como PNG en el directorio que le pases.

Antes de grabar en serio, `--dump-timeline` verifica el recorrido de camara
sin renderizar nada (rapido): imprime posicion de camara y tramo por cuadro,
y al final reporta si detecto saltos de camara fuera de los cortes de
semilla intencionales.

```
cargo run --release -- --dump-timeline --fps 30
```

## Grabar

1080p (calidad final):

```
cargo run --release -- --record frames/ --width 1920 --height 1080 --fps 30 --quality alta
```

720p (mismo guion, mas liviano/rapido de renderizar):

```
cargo run --release -- --record frames/ --width 1280 --height 720 --fps 30 --quality alta
```

- Cada cuadro sale como `frames/frame_00001.png`, `frames/frame_00002.png`, etc.
- Muestra en consola el cuadro actual, ms del cuadro y el tiempo restante
  estimado (promedio movil exponencial de los ultimos cuadros x cuadros que
  faltan), en formato `h:mm:ss`.
- Reanudable: si un PNG ya existe lo saltea sin volver a renderizarlo (podes
  cortar con Ctrl+C y correr el mismo comando de nuevo).
- Duracion total del guion: 79s. A 30fps son 2371 cuadros.

## Juntar los cuadros en un video (ffmpeg)

### 1080p para YouTube (crf 18, calidad alta, el archivo pesa lo que pese)

```
ffmpeg -framerate 30 -i frames/frame_%05d.png -c:v libx264 -pix_fmt yuv420p -crf 18 -preset slow diorama_youtube.mp4
```

### 720p para GitHub, menos de 10 MB (dos pasadas, PowerShell en Windows)

Bitrate objetivo: ~900 kbps de video (sin audio) para 79s da
`900 kbps * 79s / 8 ~= 8.9 MB`, con margen bajo el limite de 10 MB de
GitHub (con el tramo del pueblo nuevo, 79s en vez de 73s, hay menos margen
que antes -- de ahi bajar el bitrate de 1000k a 900k). Dos pasadas (mejor
reparto de bits que una sola) con `-preset slow`. La primera pasada no
necesita archivo de salida real -- en PowerShell/CMD el dispositivo nulo de
Windows es `NUL` (no `/dev/null`):

```powershell
ffmpeg -y -framerate 30 -i frames\frame_%05d.png -vf scale=1280:720 -c:v libx264 -preset slow -b:v 900k -pass 1 -an -f mp4 NUL
ffmpeg -framerate 30 -i frames\frame_%05d.png -vf scale=1280:720 -c:v libx264 -preset slow -b:v 900k -pass 2 -an diorama_github.mp4
```

(ffmpeg deja `ffmpeg2pass-0.log*` en el directorio actual, se puede borrar
despues). Si el resultado se pasa un poco de 10 MB, bajar `-b:v` (por ejemplo
a `850k`) y repetir las dos pasadas.

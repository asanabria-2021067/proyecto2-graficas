# Grabacion del video

`--record` renderiza el guion de camara de `src/record.rs` cuadro por cuadro
en maxima calidad (SSAA completo, sin el refinamiento progresivo del modo
ventana) y los guarda como PNG en el directorio que le pases:

```
cargo run --release -- --record frames/ --width 1920 --height 1080 --fps 30 --quality alta
```

- Cada cuadro sale como `frames/frame_00001.png`, `frames/frame_00002.png`, etc.
- Muestra en consola el cuadro actual, ms del cuadro y tiempo restante estimado.
- Reanudable: si un PNG ya existe lo saltea sin volver a renderizarlo (podes
  cortar con Ctrl+C y correr el mismo comando de nuevo).
- Duracion total del guion: ~74s. A 30fps son ~2221 cuadros.

## Juntar los cuadros en un video (ffmpeg)

Calidad alta (para editar/archivo):

```
ffmpeg -framerate 30 -i frames/frame_%05d.png -c:v libx264 -pix_fmt yuv420p -crf 20 diorama.mp4
```

Variante comprimida para que pese menos de 10 MB (bajando resolucion y CRF;
ajusta segun cuanto pese el resultado):

```
ffmpeg -framerate 30 -i frames/frame_%05d.png -vf scale=1280:-2 -c:v libx264 -pix_fmt yuv420p -crf 28 -preset slow -an diorama_compressed.mp4
```

Si con eso no alcanza el limite de 10 MB, dos pasadas con bitrate fijo (mas
control directo sobre el tamano final: bitrate objetivo en kbps ~=
`10*8*1024 / duracion_en_segundos`, dejando margen para el contenedor):

```
ffmpeg -y -framerate 30 -i frames/frame_%05d.png -vf scale=1280:-2 -c:v libx264 -b:v 900k -pass 1 -an -f mp4 NUL
ffmpeg -framerate 30 -i frames/frame_%05d.png -vf scale=1280:-2 -c:v libx264 -b:v 900k -pass 2 -an diorama_compressed.mp4
```

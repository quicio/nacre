# Renders de referencia

Tres renders de 2 minutos (48 kHz, estéreo, `f32`) que acompañan al change
`003-voice-synthesis`. No se versionan: `*.wav` está en `.gitignore`, y cada
archivo pesa unos 44 MiB. Se regeneran de forma determinista desde el ejemplo
offline; no hace falta dispositivo de audio.

## Cómo regenerarlos

```bash
cargo run -p nacre-audio-runtime --example render_ref -- all
cargo run -p nacre-audio-runtime --example render_ref -- drone
cargo run -p nacre-audio-runtime --example render_ref -- bass
cargo run -p nacre-audio-runtime --example render_ref -- metal
```

El proceso termina solo y escribe `assets/reference/<patch>.wav`. No reproduce
nada por el altavoz.

## Parches

| Parche | Identidad | Ajustes principales |
|---|---|---|
| `drone` | dron evolutivo | 110 Hz, saw, ataque 4 s, sustain 0,85, LFO 0,05 Hz al cutoff (35 %), cutoff 400 → 2 400 Hz en 112 s, drive 3 dB |
| `bass` | bajo pulsante | notas de 0,45 s cada 1 s sobre el patrón 55/55/49/43,65 Hz, cutoff 280 Hz, Q 5, drive 6 dB |
| `metal` | textura metálica | 220 Hz y retriggers cada 6 s (220/233,08/246,94/207,65 Hz), Q 14, LFO 0,7 Hz al cutoff (30 %), drive 12 dB |

## Medición de esta sesión (2026-11-23)

Renderizados en build `debug` con `cargo run -p nacre-audio-runtime --example
render_ref -- all` (81 s en total). `ffprobe` confirma `pcm_f32le`, 48 000 Hz,
2 canales y 120,000000 s.

| Parche | Frames | Pico | RMS | DC | Muestras a fondo de escala | Hash FNV-1a del render |
|---|---|---|---|---|---|---|
| `drone` | 5 760 000 | −5,59 dBFS | −13,84 dBFS | 2,30e−12 | 0 | `8f856bb8507da919` |
| `bass` | 5 760 000 | −6,03 dBFS | −24,29 dBFS | 5,80e−13 | 0 | `344e656bb6aae099` |
| `metal` | 5 760 000 | −7,90 dBFS | −11,56 dBFS | 1,41e−12 | 0 | `d324094d4c074635` |

Dos ejecuciones del ejemplo producen el mismo hash: el render no usa azar ni
reloj. El DC del dron (2,30e−12) queda muy por debajo del límite de 0,001.

Estos renders son material de escucha y de comparación; la escucha crítica con
un tercero (tarea 6.3) sigue pendiente.

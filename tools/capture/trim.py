#!/usr/bin/env python3
"""Corta a cabeça e a cauda de um clipe pelo som que ele mesmo contém.

Cada clipe gravado por `beats.mjs` tem uma execução musical no meio, precedida
pela navegação e pela preparação da tela. Onde a música começa não é um número
de relógio: é o primeiro som do arquivo, e o ffmpeg mede isso.

A primeira versão do reel cortava por marcas de relógio escritas pelo Playwright,
que corriam num tempo diferente do vídeo (o ffmpeg já gravava antes do navegador
existir). O offset nunca foi medido e os cortes caíam dentro das execuções.
Aqui não há offset possível: a âncora é medida no próprio artefato.

Uso:
    python3 trim.py out/b1.mkv                 # -> out/trim/b1.mp4
    python3 trim.py out/*.mkv --lead 0.25
"""
import argparse, pathlib, re, subprocess, sys

# Ruído de fundo do sampler fica bem abaixo disto; um acorde fica bem acima.
NOISE_DB = "-45dB"
MIN_SILENCE = 0.25


def probe_duration(path: pathlib.Path) -> float:
    out = subprocess.run(
        ["ffprobe", "-v", "error", "-show_entries", "format=duration",
         "-of", "csv=p=0", str(path)],
        capture_output=True, text=True, check=True).stdout.strip()
    return float(out)


def sound_window(path: pathlib.Path) -> tuple[float, float]:
    """Devolve (primeiro som, último som) em segundos."""
    proc = subprocess.run(
        ["ffmpeg", "-hide_banner", "-nostats", "-i", str(path),
         "-af", f"silencedetect=noise={NOISE_DB}:d={MIN_SILENCE}", "-f", "null", "-"],
        capture_output=True, text=True)
    log = proc.stderr
    starts = [float(m) for m in re.findall(r"silence_start: ([0-9.]+)", log)]
    ends = [float(m) for m in re.findall(r"silence_end: ([0-9.]+)", log)]
    dur = probe_duration(path)

    # Silêncio começando em ~0 significa que o clipe abre calado: o primeiro som
    # é o fim desse silêncio. Sem isso, o clipe já abre com som.
    first = ends[0] if starts and starts[0] < 0.5 and ends else 0.0
    # Cauda: se o último silêncio detectado nunca termina, ele vai até o fim.
    last = starts[-1] if starts and (not ends or starts[-1] > ends[-1]) else dur
    return first, last


def trim(path: pathlib.Path, lead: float, tail: float, outdir: pathlib.Path) -> None:
    first, last = sound_window(path)
    start = max(0.0, first - lead)
    end = min(probe_duration(path), last + tail)
    outdir.mkdir(parents=True, exist_ok=True)
    out = outdir / f"{path.stem}.mp4"

    subprocess.run(
        ["ffmpeg", "-hide_banner", "-loglevel", "error", "-y",
         "-ss", f"{start:.3f}", "-to", f"{end:.3f}", "-i", str(path),
         # Re-encode: corte exato no frame, e o Motion Canvas lê mp4/h264 sem drama.
         "-c:v", "libx264", "-preset", "slow", "-crf", "17", "-pix_fmt", "yuv420p",
         "-c:a", "aac", "-b:a", "192k", "-movflags", "+faststart", str(out)],
        check=True)
    print(f"{path.name}: som {first:.2f}s a {last:.2f}s -> {out.name} "
          f"({end - start:.2f}s, corte em {start:.2f})")


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("clips", nargs="+", type=pathlib.Path)
    ap.add_argument("--lead", type=float, default=0.30,
                    help="segundos de respiro antes do primeiro som")
    ap.add_argument("--tail", type=float, default=0.80,
                    help="segundos de cauda depois do último som")
    ap.add_argument("--outdir", type=pathlib.Path, default=None)
    args = ap.parse_args()

    for clip in args.clips:
        if not clip.exists():
            print(f"nao existe: {clip}", file=sys.stderr)
            continue
        outdir = args.outdir or clip.parent / "trim"
        trim(clip, args.lead, args.tail, outdir)


if __name__ == "__main__":
    main()

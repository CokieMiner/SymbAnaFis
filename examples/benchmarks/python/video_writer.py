"""Video writer helpers for Matplotlib animations.

Probes NVIDIA NVENC with a 1-frame test encode and falls back cleanly to fast
libx264 (`yuv420p`) so exports never crash on hosts without an NVIDIA GPU.
"""

from __future__ import annotations

import shutil
import subprocess
from pathlib import Path


def _has_ffmpeg() -> bool:
    return shutil.which("ffmpeg") is not None


def _has_working_nvenc() -> bool:
    if not _has_ffmpeg():
        return False
    try:
        proc = subprocess.run(
            [
                "ffmpeg",
                "-hide_banner",
                "-loglevel",
                "error",
                "-f",
                "lavfi",
                "-i",
                "color=black:s=64x64:d=0.04",
                "-frames:v",
                "1",
                "-c:v",
                "h264_nvenc",
                "-f",
                "null",
                "-",
            ],
            capture_output=True,
            text=True,
            check=False,
        )
        return proc.returncode == 0
    except Exception:
        return False


def save_animation_mp4(
    ani,
    out_path: Path,
    fps: int = 30,
    dpi: int = 100,
    cq: int = 24,
) -> str:
    """Save animation using NVENC when available, otherwise fast libx264.

    Returns:
        "GPU" if NVENC was used, else "CPU".
    """
    from matplotlib.animation import FFMpegWriter

    out_path.parent.mkdir(parents=True, exist_ok=True)

    if _has_working_nvenc():
        try:
            writer = FFMpegWriter(
                fps=fps,
                codec="h264_nvenc",
                extra_args=[
                    "-preset",
                    "p4",
                    "-rc",
                    "vbr",
                    "-cq",
                    str(cq),
                    "-pix_fmt",
                    "yuv420p",
                    "-movflags",
                    "+faststart",
                ],
            )
            ani.save(str(out_path), writer=writer, dpi=dpi)
            return "GPU"
        except Exception as exc:
            print(f"NVENC encode failed ({exc}); falling back to CPU libx264.")

    writer = FFMpegWriter(
        fps=fps,
        codec="libx264",
        extra_args=[
            "-preset",
            "fast",
            "-crf",
            str(min(cq, 24)),
            "-pix_fmt",
            "yuv420p",
            "-movflags",
            "+faststart",
        ],
    )
    ani.save(str(out_path), writer=writer, dpi=dpi)
    return "CPU"

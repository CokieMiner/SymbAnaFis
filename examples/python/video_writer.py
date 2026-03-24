"""Video writer helpers for Matplotlib animations.

Uses NVIDIA NVENC when available and falls back to libx264 with an explicit log.
"""

from __future__ import annotations

import shutil
import subprocess
from pathlib import Path


def _has_ffmpeg() -> bool:
    return shutil.which("ffmpeg") is not None


def _has_nvenc_encoder() -> bool:
    if not _has_ffmpeg():
        return False
    try:
        proc = subprocess.run(
            ["ffmpeg", "-hide_banner", "-encoders"],
            capture_output=True,
            text=True,
            check=False,
        )
        return "h264_nvenc" in proc.stdout
    except Exception:
        return False


def save_animation_mp4(
    ani,
    out_path: Path,
    fps: int = 30,
    dpi: int = 100,
    cq: int = 24,
) -> str:
    """Save animation using NVENC when available.

    Returns:
        "GPU" if NVENC was used, else "CPU".
    """
    from matplotlib.animation import FFMpegWriter

    out_path.parent.mkdir(exist_ok=True)

    if _has_nvenc_encoder():
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
                "-movflags",
                "+faststart",
            ],
        )
        ani.save(str(out_path), writer=writer, dpi=dpi)
        return "GPU"

    writer = FFMpegWriter(
        fps=fps,
        codec="libx264",
        extra_args=["-preset", "medium", "-crf", "23", "-movflags", "+faststart"],
    )
    ani.save(str(out_path), writer=writer, dpi=dpi)
    return "CPU"

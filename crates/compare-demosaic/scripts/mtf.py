#!/usr/bin/env python3
"""ISO 12233-style slanted-edge MTF analysis and SVG chart generation."""

from __future__ import annotations

import math
from dataclasses import dataclass
from pathlib import Path
from typing import Annotated

import matplotlib.pyplot as plt
import numpy as np
import tifffile
import typer
from chart_spec import BAYER_PATTERNS, TILES

CRATE_DIR = Path(__file__).resolve().parent.parent
REPO_ROOT = CRATE_DIR.parent.parent
DEFAULT_TEMP_DIR = REPO_ROOT / "temp" / "compare-demosaic"
DEFAULT_DATA_DIR = CRATE_DIR / "data"
ALL_METHODS = ("linear", "ppg")

app = typer.Typer(no_args_is_help=True)


@dataclass(frozen=True)
class EdgeSpec:
    """Definition of a 5-degree slanted-edge sub-ROI inside the 4x4 chart."""

    id: str
    title: str
    tile_id: str
    quadrant: str  # "tl" | "tr" | "bl"
    channels: tuple[str, ...]  # ("Y", "R", "G", "B")
    margin: int = 16

    def patch_slices(self) -> tuple[slice, slice]:
        """Return (y_slice, x_slice) for the trimmed slanted-edge sub-ROI."""
        tile = next(t for t in TILES if t.id == self.tile_id)
        x0, y0, size, _ = tile.roi_bounds()
        half = size // 2
        if self.quadrant == "tl":
            qx, qy = x0, y0
        elif self.quadrant == "tr":
            qx, qy = x0 + half, y0
        elif self.quadrant == "bl":
            qx, qy = x0, y0 + half
        else:
            msg = f"Unsupported quadrant: {self.quadrant}"
            raise ValueError(msg)
        return (
            slice(qy + self.margin, qy + half - self.margin),
            slice(qx + self.margin, qx + half - self.margin),
        )


EDGE_SPECS: tuple[EdgeSpec, ...] = (
    EdgeSpec(
        id="achromatic_bw",
        title="Achromatic B/W 5 deg Edge ([0, 1])",
        tile_id="line_pairs_edges",
        quadrant="tl",
        channels=("Y", "R", "G", "B"),
    ),
    EdgeSpec(
        id="chromatic_rb",
        title="Chromatic R/B 5 deg Edge ([0, 1])",
        tile_id="line_pairs_edges",
        quadrant="tr",
        channels=("R", "B"),
    ),
    EdgeSpec(
        id="signed_zero_crossing",
        title="Signed Zero-Crossing Edge ([-0.25, +0.25])",
        tile_id="signed_zero_crossing",
        quadrant="bl",
        channels=("Y",),
    ),
    EdgeSpec(
        id="wide_range_hdr",
        title="Wide-Range Signed-HDR Edge ([-0.5, +2.5])",
        tile_id="wide_range_signed_hdr",
        quadrant="tl",
        channels=("Y",),
    ),
)


@dataclass
class MtfResult:
    """Slanted-edge SFR / MTF, ESF, LSF, and scalar summary metrics."""

    freqs: np.ndarray
    mtf: np.ndarray
    esf_x: np.ndarray
    esf: np.ndarray
    lsf_x: np.ndarray
    lsf: np.ndarray
    mtf50: float
    mtf20: float
    mtf_025: float
    mtf_050: float
    mtf_peak: float
    slant_deg: float


def _extract_channel(rgb: np.ndarray, channel: str) -> np.ndarray:
    if channel == "Y":
        return (
            0.2126 * rgb[..., 0].astype(np.float64)
            + 0.7152 * rgb[..., 1].astype(np.float64)
            + 0.0722 * rgb[..., 2].astype(np.float64)
        )
    idx = {"R": 0, "G": 1, "B": 2}[channel]
    return rgb[..., idx].astype(np.float64)


def _crossing_freq(freqs: np.ndarray, mtf: np.ndarray, level: float) -> float:
    """Find the first frequency where MTF crosses below `level`."""
    for i in range(1, len(freqs)):
        if mtf[i] <= level < mtf[i - 1]:
            denom = mtf[i - 1] - mtf[i]
            t = (mtf[i - 1] - level) / denom if denom > 0 else 0.0
            return float(freqs[i - 1] + t * (freqs[i] - freqs[i - 1]))
    return float(freqs[-1]) if mtf[-1] > level else 0.0


def compute_slanted_edge_mtf(
    patch: np.ndarray,
    oversample: int = 4,
    half_window_px: int = 24,
) -> MtfResult:
    """Compute ISO 12233 slanted-edge MTF, ESF, and LSF from a 2D image patch."""
    img = np.asarray(patch, dtype=np.float64)
    h, w = img.shape

    # Normalize step polarity so rising and falling edges share positive LSF polarity.
    left_mean = float(np.mean(img[:, : max(1, w // 4)]))
    right_mean = float(np.mean(img[:, -max(1, w // 4) :]))
    sign = 1.0 if right_mean >= left_mean else -1.0
    oriented = img * sign

    # Row-wise gradient centroids around median peak column.
    diff = oriented[:, 1:] - oriented[:, :-1]
    peak_cols = np.argmax(np.abs(diff), axis=1)
    med_col = int(np.median(peak_cols))
    c0 = max(0, med_col - 12)
    c1 = min(w - 1, med_col + 13)
    x_cols = np.arange(c0, c1, dtype=np.float64) + 0.5
    local_weights = np.maximum(diff[:, c0:c1], 0.0)
    if float(np.sum(local_weights)) <= 1e-12:
        local_weights = np.abs(diff[:, c0:c1])

    row_sums = np.sum(local_weights, axis=1)
    valid_rows = row_sums > 1e-12
    y_idx = np.arange(h, dtype=np.float64)[valid_rows]
    x_c = np.sum(local_weights[valid_rows] * x_cols[None, :], axis=1) / row_sums[valid_rows]

    slope, intercept = np.polyfit(y_idx, x_c, 1)
    cos_theta = 1.0 / math.hypot(1.0, float(slope))
    slant_deg = math.degrees(math.atan(float(slope)))

    # Project all pixels to perpendicular signed distance from fitted edge.
    yy, xx = np.mgrid[:h, :w]
    d_perp = (xx - (slope * yy + intercept)) * cos_theta

    ds = 1.0 / oversample
    n_bins = 2 * half_window_px * oversample
    bin_edges = np.linspace(-half_window_px, half_window_px, n_bins + 1)
    esf_x = 0.5 * (bin_edges[:-1] + bin_edges[1:])

    flat_d = d_perp.ravel()
    flat_v = oriented.ravel()
    in_range = (flat_d >= -half_window_px) & (flat_d < half_window_px)
    bin_idx = np.floor((flat_d[in_range] + half_window_px) / ds).astype(np.intp)
    bin_idx = np.clip(bin_idx, 0, n_bins - 1)

    sums = np.bincount(bin_idx, weights=flat_v[in_range], minlength=n_bins)
    counts = np.bincount(bin_idx, minlength=n_bins)
    populated = counts > 0
    esf = np.empty(n_bins, dtype=np.float64)
    esf[populated] = sums[populated] / counts[populated]
    if not np.all(populated):
        esf[~populated] = np.interp(esf_x[~populated], esf_x[populated], esf[populated])

    # Normalize ESF to [0, 1] scale for clean comparison across signed/HDR ranges.
    esf_lo = float(np.mean(esf[: oversample * 4]))
    esf_hi = float(np.mean(esf[-oversample * 4 :]))
    esf_span = esf_hi - esf_lo
    esf_norm = (esf - esf_lo) / esf_span if abs(esf_span) > 1e-12 else esf - esf_lo

    # Centered finite-difference LSF + Hann window centered on LSF peak.
    lsf = 0.5 * (esf_norm[2:] - esf_norm[:-2])
    lsf_x = esf_x[1:-1]
    peak_idx = int(np.argmax(np.abs(lsf)))
    half_win_bins = min(peak_idx, len(lsf) - 1 - peak_idx)
    window = np.zeros_like(lsf)
    if half_win_bins >= 4:
        win_len = 2 * half_win_bins + 1
        window[peak_idx - half_win_bins : peak_idx + half_win_bins + 1] = np.hanning(win_len)
    else:
        window[:] = np.hanning(len(lsf))
    windowed_lsf = lsf * window

    spec = np.abs(np.fft.rfft(windowed_lsf))
    dc = float(spec[0]) if spec[0] > 1e-12 else 1.0
    mtf_raw = spec / dc
    freqs_all = np.fft.rfftfreq(len(windowed_lsf), d=ds)

    # ISO 12233 centered-difference transfer correction: (2*pi*f*ds) / sin(2*pi*f*ds).
    keep = freqs_all <= 0.60
    freqs = freqs_all[keep]
    mtf = mtf_raw[keep].copy()
    nz = freqs > 0.0
    arg = 2.0 * math.pi * freqs[nz] * ds
    mtf[nz] /= np.sin(arg) / arg

    nyq_mask = freqs <= 0.50
    mtf50 = _crossing_freq(freqs, mtf, 0.50)
    mtf20 = _crossing_freq(freqs, mtf, 0.20)
    mtf_025 = float(np.interp(0.25, freqs, mtf))
    mtf_050 = float(np.interp(0.50, freqs, mtf))
    mtf_peak = float(np.max(mtf[nyq_mask])) if np.any(nyq_mask) else float(np.max(mtf))

    return MtfResult(
        freqs=freqs,
        mtf=mtf,
        esf_x=esf_x,
        esf=esf_norm,
        lsf_x=lsf_x,
        lsf=lsf,
        mtf50=mtf50,
        mtf20=mtf20,
        mtf_025=mtf_025,
        mtf_050=mtf_050,
        mtf_peak=mtf_peak,
        slant_deg=slant_deg,
    )


def _compute_image_edges(rgb: np.ndarray) -> dict[tuple[str, str], MtfResult]:
    results: dict[tuple[str, str], MtfResult] = {}
    for edge in EDGE_SPECS:
        ys, xs = edge.patch_slices()
        patch_rgb = rgb[ys, xs, :]
        for ch in edge.channels:
            results[(edge.id, ch)] = compute_slanted_edge_mtf(_extract_channel(patch_rgb, ch))
    return results


def _setup_mtf_axes(ax: plt.Axes, title: str, ylabel: str = "MTF") -> None:
    ax.set_title(title, fontsize=10, fontweight="bold")
    ax.set_xlabel("Spatial Frequency (cycles/pixel)", fontsize=8.5)
    ax.set_ylabel(ylabel, fontsize=8.5)
    ax.set_xlim(0.0, 0.55)
    ax.set_ylim(0.0, 1.15)
    ax.axvline(0.50, color="#6b7280", linestyle=":", linewidth=1.0, label="Nyquist (0.50)")
    ax.axhline(0.50, color="#9ca3af", linestyle="--", linewidth=0.9)
    ax.grid(True, linestyle=":", alpha=0.5)


def _plot_gt_halo(ax: plt.Axes, freqs: np.ndarray, mtf: np.ndarray, label: str) -> None:
    """Plot Ground Truth as a wide translucent reference band + thin center dashed hairline."""
    ax.plot(freqs, mtf, color="#9ca3af", linewidth=5.0, alpha=0.45, zorder=1)
    ax.plot(freqs, mtf, color="#374151", linestyle="--", linewidth=1.0, zorder=2, label=label)


def _plot_series(
    ax: plt.Axes,
    freqs: np.ndarray,
    values: np.ndarray,
    *,
    color: str,
    linestyle: str,
    marker: str,
    markevery: tuple[int, int],
    label: str,
    linewidth: float = 1.6,
    zorder: int = 3,
) -> None:
    ax.plot(
        freqs,
        values,
        color=color,
        linestyle=linestyle,
        linewidth=linewidth,
        marker=marker,
        markersize=5.0,
        markevery=markevery,
        markeredgecolor="white",
        markeredgewidth=0.6,
        zorder=zorder,
        label=label,
    )


def plot_method_mtf(
    output_svg: Path,
    method: str,
    pattern: str,
    est_results: dict[tuple[str, str], MtfResult],
    gt_results: dict[tuple[str, str], MtfResult],
) -> None:
    """Write a 2x3 (6-panel) SVG MTF + residual + ESF/LSF chart for one (method, pattern)."""
    output_svg.parent.mkdir(parents=True, exist_ok=True)
    fig, axes = plt.subplots(2, 3, figsize=(15.5, 9.2), dpi=150, constrained_layout=True)
    fig.suptitle(
        f"Slanted-Edge MTF — {method} ({pattern.upper()}) vs. Ground Truth",
        fontsize=12.5,
        fontweight="bold",
    )

    ch_styles = {
        "Y": ("#0f766e", "-", "D", (0, 4)),
        "R": ("#dc2626", "--", "o", (1, 4)),
        "G": ("#16a34a", "-.", "^", (2, 4)),
        "B": ("#2563eb", ":", "s", (3, 4)),
    }

    # Panel (0,0): Achromatic B/W edge (Y, R, G, B)
    ax = axes[0, 0]
    _setup_mtf_axes(ax, "Achromatic 5 deg Edge (Y / R / G / B)")
    gt_y = gt_results[("achromatic_bw", "Y")]
    _plot_gt_halo(ax, gt_y.freqs, gt_y.mtf, f"GT Y (MTF50={gt_y.mtf50:.3f})")
    for ch in ("Y", "R", "G", "B"):
        col, ls, mk, me = ch_styles[ch]
        res = est_results[("achromatic_bw", ch)]
        _plot_series(
            ax,
            res.freqs,
            res.mtf,
            color=col,
            linestyle=ls,
            marker=mk,
            markevery=me,
            label=f"{ch} (MTF50={res.mtf50:.3f})",
        )
    ax.legend(fontsize=7.2, loc="lower left")

    # Panel (0,1): Achromatic MTF residual (MTF - MTF_GT)
    ax = axes[0, 1]
    ax.set_title("Achromatic MTF Residual (Est - GT)", fontsize=10, fontweight="bold")
    ax.set_xlabel("Spatial Frequency (cycles/pixel)", fontsize=8.5)
    ax.set_ylabel("Delta MTF", fontsize=8.5)
    ax.set_xlim(0.0, 0.55)
    ax.axvline(0.50, color="#6b7280", linestyle=":", linewidth=1.0, label="Nyquist (0.50)")
    ax.axhline(0.0, color="#374151", linestyle="--", linewidth=1.0, label="Zero (GT)")
    ax.grid(True, linestyle=":", alpha=0.5)
    resids = []
    for ch in ("Y", "R", "G", "B"):
        col, ls, mk, me = ch_styles[ch]
        res = est_results[("achromatic_bw", ch)]
        gt_ch = gt_results[("achromatic_bw", ch)]
        delta = res.mtf - gt_ch.mtf
        resids.append(delta)
        _plot_series(
            ax,
            res.freqs,
            delta,
            color=col,
            linestyle=ls,
            marker=mk,
            markevery=me,
            label=f"{ch} (max={float(np.max(np.abs(delta))):+.3f})",
        )
    max_abs = max(0.03, float(np.max(np.abs(np.concatenate(resids)))))
    ax.set_ylim(-max_abs * 1.15, max_abs * 1.15)
    ax.legend(fontsize=7.2, loc="lower left")

    # Panel (0,2): Chromatic R/B edge
    ax = axes[0, 2]
    _setup_mtf_axes(ax, "Chromatic R/B 5 deg Edge (R & B)")
    gt_r = gt_results[("chromatic_rb", "R")]
    _plot_gt_halo(ax, gt_r.freqs, gt_r.mtf, f"GT R/B (MTF50={gt_r.mtf50:.3f})")
    for ch, col, ls, mk, me in (
        ("R", "#dc2626", "-", "o", (0, 3)),
        ("B", "#2563eb", "--", "s", (1, 3)),
    ):
        res = est_results[("chromatic_rb", ch)]
        _plot_series(
            ax,
            res.freqs,
            res.mtf,
            color=col,
            linestyle=ls,
            marker=mk,
            markevery=me,
            label=f"{method} {ch} (MTF50={res.mtf50:.3f})",
        )
    ax.legend(fontsize=7.5, loc="lower left")

    # Panel (1,0): Unclamped signed & HDR edges
    ax = axes[1, 0]
    _setup_mtf_axes(ax, "Unclamped Float Edges (Signed & HDR)")
    gt_s = gt_results[("signed_zero_crossing", "Y")]
    _plot_gt_halo(ax, gt_s.freqs, gt_s.mtf, f"GT (MTF50={gt_s.mtf50:.3f})")
    res_s = est_results[("signed_zero_crossing", "Y")]
    res_h = est_results[("wide_range_hdr", "Y")]
    _plot_series(
        ax,
        res_s.freqs,
        res_s.mtf,
        color="#7c3aed",
        linestyle="-",
        marker="o",
        markevery=(0, 3),
        label=f"Signed [-0.25,+0.25] ({res_s.mtf50:.3f})",
    )
    _plot_series(
        ax,
        res_h.freqs,
        res_h.mtf,
        color="#d97706",
        linestyle="--",
        marker="s",
        markevery=(1, 3),
        label=f"HDR [-0.5,+2.5] ({res_h.mtf50:.3f})",
    )
    diff_sh = float(np.max(np.abs(res_s.mtf - res_h.mtf)))
    ax.text(
        0.97,
        0.95,
        f"max |Signed - HDR| = {diff_sh:.4f}",
        transform=ax.transAxes,
        ha="right",
        va="top",
        fontsize=7.5,
        bbox={"boxstyle": "round,pad=0.25", "facecolor": "white", "alpha": 0.85, "edgecolor": "#d1d5db"},
    )
    ax.legend(fontsize=7.2, loc="lower left")

    # Panel (1,1): Achromatic ESF zoom [-6, +6] px
    ax = axes[1, 1]
    ax.set_title("Achromatic 4x-Oversampled ESF", fontsize=10, fontweight="bold")
    ax.set_xlabel("Perpendicular Distance from Edge (pixels)", fontsize=8.5)
    ax.set_ylabel("Normalized Amplitude", fontsize=8.5)
    ax.set_xlim(-6.0, 6.0)
    ax.set_ylim(-0.15, 1.15)
    ax.grid(True, linestyle=":", alpha=0.5)
    est_y = est_results[("achromatic_bw", "Y")]
    ax.plot(gt_y.esf_x, gt_y.esf, color="#9ca3af", linewidth=5.0, alpha=0.45, zorder=1)
    ax.plot(gt_y.esf_x, gt_y.esf, color="#374151", linestyle="--", linewidth=1.0, zorder=2, label="GT ESF")
    _plot_series(
        ax,
        est_y.esf_x,
        est_y.esf,
        color="#0f766e",
        linestyle="-",
        marker="o",
        markevery=(0, 2),
        label=f"{method} ESF",
    )
    ax.legend(fontsize=7.5, loc="upper left")

    # Panel (1,2): Achromatic LSF zoom [-6, +6] px
    ax = axes[1, 2]
    ax.set_title("Achromatic 4x-Oversampled LSF (Normalized)", fontsize=10, fontweight="bold")
    ax.set_xlabel("Perpendicular Distance from Edge (pixels)", fontsize=8.5)
    ax.set_ylabel("Normalized LSF", fontsize=8.5)
    ax.set_xlim(-6.0, 6.0)
    ax.set_ylim(-0.20, 1.15)
    ax.axhline(0.0, color="#9ca3af", linestyle=":", linewidth=0.9)
    ax.grid(True, linestyle=":", alpha=0.5)
    gt_lsf_norm = gt_y.lsf / max(float(np.max(np.abs(gt_y.lsf))), 1e-6)
    est_lsf_norm = est_y.lsf / max(float(np.max(np.abs(est_y.lsf))), 1e-6)
    ax.fill_between(gt_y.lsf_x, 0.0, gt_lsf_norm, color="#9ca3af", alpha=0.35, zorder=1)
    ax.plot(gt_y.lsf_x, gt_lsf_norm, color="#4b5563", linestyle="--", linewidth=1.0, zorder=2, label="GT LSF")
    _plot_series(
        ax,
        est_y.lsf_x,
        est_lsf_norm,
        color="#dc2626",
        linestyle="-",
        marker="o",
        markevery=(0, 2),
        label=f"{method} LSF",
    )
    ax.legend(fontsize=7.5, loc="upper left")

    fig.savefig(output_svg, format="svg", bbox_inches="tight")
    plt.close(fig)


def plot_comparison_mtf(
    output_svg: Path,
    method_curves: dict[str, dict[tuple[str, str], list[MtfResult]]],
    gt_results: dict[tuple[str, str], MtfResult],
) -> None:
    """Write a combined 2x3 (6-panel) SVG MTF chart comparing all methods (mean over 4 Bayer patterns)."""
    output_svg.parent.mkdir(parents=True, exist_ok=True)
    fig, axes = plt.subplots(2, 3, figsize=(15.5, 9.2), dpi=150, constrained_layout=True)
    fig.suptitle(
        "Slanted-Edge MTF Comparison (Mean Across 4 Bayer Patterns)",
        fontsize=12.5,
        fontweight="bold",
    )
    method_styles = {
        "linear": ("#2563eb", "-", "o", 0),
        "ppg": ("#dc2626", "--", "s", 1),
    }

    def mean_curve(m: str, edge_id: str, ch: str) -> tuple[np.ndarray, np.ndarray, float]:
        items = method_curves[m][(edge_id, ch)]
        freqs = items[0].freqs
        mtf_mean = np.mean([r.mtf for r in items], axis=0)
        mtf50_mean = float(np.mean([r.mtf50 for r in items]))
        return freqs, mtf_mean, mtf50_mean

    # Panel (0,0): Achromatic Luma Y
    ax = axes[0, 0]
    _setup_mtf_axes(ax, "Achromatic 5 deg Edge — Luma (Y)")
    gt_y = gt_results[("achromatic_bw", "Y")]
    _plot_gt_halo(ax, gt_y.freqs, gt_y.mtf, f"Ground Truth (MTF50={gt_y.mtf50:.3f})")
    for m in method_curves:
        col, ls, mk, phase = method_styles.get(m, ("#0f766e", "-", "D", 2))
        freqs, mtf_m, mtf50_m = mean_curve(m, "achromatic_bw", "Y")
        _plot_series(
            ax,
            freqs,
            mtf_m,
            color=col,
            linestyle=ls,
            marker=mk,
            markevery=(phase, 3),
            linewidth=1.8,
            label=f"{m} (MTF50={mtf50_m:.3f})",
        )
    ax.legend(fontsize=7.5, loc="lower left")

    # Panel (0,1): Achromatic Per-Channel (R / G / B)
    ax = axes[0, 1]
    _setup_mtf_axes(ax, "Achromatic 5 deg Edge — Per-Channel (R / G / B)")
    _plot_gt_halo(ax, gt_y.freqs, gt_y.mtf, f"Ground Truth ({gt_y.mtf50:.3f})")
    ch_colors = {"R": "#dc2626", "G": "#16a34a", "B": "#2563eb"}
    ch_markers = {"R": "o", "G": "^", "B": "s"}
    m_linestyles = {"linear": ":", "ppg": "-"}
    for m_idx, m in enumerate(method_curves):
        for ch_idx, ch in enumerate(("R", "G", "B")):
            freqs, mtf_m, mtf50_m = mean_curve(m, "achromatic_bw", ch)
            phase = (ch_idx + 2 * m_idx) % 5
            _plot_series(
                ax,
                freqs,
                mtf_m,
                color=ch_colors[ch],
                linestyle=m_linestyles.get(m, "-."),
                marker=ch_markers[ch],
                markevery=(phase, 5),
                linewidth=1.5,
                label=f"{m} {ch} ({mtf50_m:.3f})",
            )
    ax.legend(fontsize=6.8, loc="lower left", ncol=2)

    # Panel (0,2): Chromatic R/B edge
    ax = axes[0, 2]
    _setup_mtf_axes(ax, "Chromatic R/B 5 deg Edge (R & B)")
    gt_r = gt_results[("chromatic_rb", "R")]
    _plot_gt_halo(ax, gt_r.freqs, gt_r.mtf, f"Ground Truth ({gt_r.mtf50:.3f})")
    rb_styles = {
        ("linear", "R"): ("#1d4ed8", "-", "o", 0),
        ("linear", "B"): ("#60a5fa", "--", "s", 1),
        ("ppg", "R"): ("#b91c1c", "-.", "^", 2),
        ("ppg", "B"): ("#f97316", ":", "D", 3),
    }
    for m in method_curves:
        for ch in ("R", "B"):
            col, ls, mk, phase = rb_styles.get((m, ch), ("#0f766e", "-", "o", 0))
            freqs, mtf_m, mtf50_m = mean_curve(m, "chromatic_rb", ch)
            _plot_series(
                ax,
                freqs,
                mtf_m,
                color=col,
                linestyle=ls,
                marker=mk,
                markevery=(phase, 4),
                label=f"{m} {ch} ({mtf50_m:.3f})",
            )
    ax.legend(fontsize=7.2, loc="lower left")

    # Panel (1,0): Achromatic Luma Residual (MTF_Y - MTF_GT)
    ax = axes[1, 0]
    ax.set_title("Achromatic Luma Residual (MTF_Y - MTF_GT)", fontsize=10, fontweight="bold")
    ax.set_xlabel("Spatial Frequency (cycles/pixel)", fontsize=8.5)
    ax.set_ylabel("Delta MTF", fontsize=8.5)
    ax.set_xlim(0.0, 0.55)
    ax.axvline(0.50, color="#6b7280", linestyle=":", linewidth=1.0, label="Nyquist (0.50)")
    ax.axhline(0.0, color="#374151", linestyle="--", linewidth=1.0, label="Zero (GT)")
    ax.grid(True, linestyle=":", alpha=0.5)
    resids = []
    for m in method_curves:
        col, ls, mk, phase = method_styles.get(m, ("#0f766e", "-", "D", 2))
        freqs, mtf_m, _ = mean_curve(m, "achromatic_bw", "Y")
        delta = mtf_m - gt_y.mtf
        resids.append(delta)
        _plot_series(
            ax,
            freqs,
            delta,
            color=col,
            linestyle=ls,
            marker=mk,
            markevery=(phase, 3),
            linewidth=1.8,
            label=f"{m} (at 0.50: {float(np.interp(0.50, freqs, delta)):+.3f})",
        )
    max_abs = max(0.03, float(np.max(np.abs(np.concatenate(resids)))))
    ax.set_ylim(-max_abs * 1.15, max_abs * 1.15)
    ax.legend(fontsize=7.5, loc="lower left")

    # Panel (1,1): Unclamped Signed Edge [-0.25, +0.25]
    ax = axes[1, 1]
    _setup_mtf_axes(ax, "Unclamped Signed Edge ([-0.25, +0.25])")
    gt_s = gt_results[("signed_zero_crossing", "Y")]
    _plot_gt_halo(ax, gt_s.freqs, gt_s.mtf, f"Ground Truth ({gt_s.mtf50:.3f})")
    for m in method_curves:
        col, ls, mk, phase = method_styles.get(m, ("#0f766e", "-", "D", 2))
        freqs, mtf_m, mtf50_m = mean_curve(m, "signed_zero_crossing", "Y")
        _plot_series(
            ax,
            freqs,
            mtf_m,
            color=col,
            linestyle=ls,
            marker=mk,
            markevery=(phase, 3),
            linewidth=1.8,
            label=f"{m} ({mtf50_m:.3f})",
        )
    ax.legend(fontsize=7.5, loc="lower left")

    # Panel (1,2): Unclamped Wide-Range HDR Edge [-0.5, +2.5]
    ax = axes[1, 2]
    _setup_mtf_axes(ax, "Unclamped Wide-Range HDR Edge ([-0.5, +2.5])")
    gt_h = gt_results[("wide_range_hdr", "Y")]
    _plot_gt_halo(ax, gt_h.freqs, gt_h.mtf, f"Ground Truth ({gt_h.mtf50:.3f})")
    for m in method_curves:
        col, ls, mk, phase = method_styles.get(m, ("#0f766e", "-", "D", 2))
        freqs, mtf_m, mtf50_m = mean_curve(m, "wide_range_hdr", "Y")
        _plot_series(
            ax,
            freqs,
            mtf_m,
            color=col,
            linestyle=ls,
            marker=mk,
            markevery=(phase, 3),
            linewidth=1.8,
            label=f"{m} ({mtf50_m:.3f})",
        )
    ax.legend(fontsize=7.5, loc="lower left")

    fig.savefig(output_svg, format="svg", bbox_inches="tight")
    plt.close(fig)


def evaluate_mtf(
    temp_dir: Path,
    methods: list[str],
    gt_rgb: np.ndarray | None = None,
    write_charts: bool = True,
) -> list[dict[str, float | str]]:
    """Compute MTF metrics and generate SVG MTF charts for all requested methods."""
    if gt_rgb is None:
        gt_rgb = tifffile.imread(temp_dir / "ground_truth_rgb.tif").astype(np.float32)

    gt_results = _compute_image_edges(gt_rgb)
    mtf_rows: list[dict[str, float | str]] = []
    method_curves: dict[str, dict[tuple[str, str], list[MtfResult]]] = {}

    for method in methods:
        method_dir = temp_dir / method
        curves: dict[tuple[str, str], list[MtfResult]] = {(e.id, ch): [] for e in EDGE_SPECS for ch in e.channels}
        for pattern in BAYER_PATTERNS:
            est_rgb = tifffile.imread(method_dir / f"demosaic_{pattern}.tif").astype(np.float32)
            est_results = _compute_image_edges(est_rgb)
            for key, res in est_results.items():
                curves[key].append(res)
                edge_id, ch = key
                mtf_rows.append(
                    {
                        "method": method,
                        "pattern": pattern,
                        "edge_id": edge_id,
                        "channel": ch,
                        "mtf50": res.mtf50,
                        "mtf20": res.mtf20,
                        "mtf_025": res.mtf_025,
                        "mtf_050": res.mtf_050,
                        "mtf_peak": res.mtf_peak,
                        "slant_deg": res.slant_deg,
                    }
                )
            if write_charts:
                plot_method_mtf(method_dir / f"mtf_{pattern}.svg", method, pattern, est_results, gt_results)
        method_curves[method] = curves

    if write_charts and method_curves:
        plot_comparison_mtf(temp_dir / "mtf_comparison.svg", method_curves, gt_results)

    return mtf_rows


@app.command()
def main(
    method: Annotated[str, typer.Option("--method", help="Method(s): linear, ppg, all, or comma-separated.")] = "all",
    temp_dir: Annotated[Path, typer.Option("--temp-dir", help="Temp directory with TIFFs.")] = DEFAULT_TEMP_DIR,
) -> None:
    """Compute slanted-edge MTF metrics and generate SVG MTF charts."""
    tokens = [tok.strip().lower() for tok in method.split(",") if tok.strip()]
    if not tokens or "all" in tokens:
        methods = list(ALL_METHODS)
    else:
        methods = []
        for tok in tokens:
            if tok not in ALL_METHODS:
                msg = f"Unsupported method '{tok}'; expected one of {ALL_METHODS} or 'all'"
                raise typer.BadParameter(msg)
            if tok not in methods:
                methods.append(tok)

    rows = evaluate_mtf(temp_dir=temp_dir, methods=methods, write_charts=True)
    typer.echo(f"generated MTF SVG charts in {temp_dir} ({len(rows)} edge measurements)")


if __name__ == "__main__":
    app()

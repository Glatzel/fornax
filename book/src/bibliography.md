# Bibliography

Use stable identifiers in chapter prose. For example, `[FornaxArchitecture]`
links to the repository reference below.

## Repository references

### [FornaxArchitecture]

- Source: [docs/architecture/overview.md](../../docs/architecture/overview.md)
- Supports: crate roles, pipeline boundaries, metadata flow, and the supported
  2x2 RGB Bayer scope.
- Note: this is the repository's concise implementation map; the book explains
  the underlying ideas rather than duplicating it.

### [FornaxComparison]

- Source:
  [crates/compare-demosaic/README.md](../../crates/compare-demosaic/README.md)
- Supports: reference-versus-Fornax comparison commands, metric definitions,
  verdicts, and accepted-outlier rules.
- Note: `baseline/canonical.json` is the source of truth for measured results.

### [VNGNotes]

- Source: [docs/reference/algorithms/vng.md](../../docs/reference/algorithms/vng.md)
- Supports: Fornax's VNG algorithm shape, green-plane convention, expected
  comparison behavior, and source licensing constraints.

## External references

### [Chen1999]

- Source:
  [Interpolation using a Threshold-based variable number of gradients](http://scien.stanford.edu/pages/labsite/1999/psych221/projects/99/tingchen/algodep/vargra.html)
- Supports: the original published description of VNG.
- Accessed: 2026-11-03.

### [DarktableVNG562]

- Source:
  [darktable 5.6.2 `vng_interpolate`](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/vng.c#L171)
- Supports: the acceptance reference for the darktable-compatible Fornax VNG
  implementation.
- Licence: darktable is GPL. Use it as behavioral and specification evidence;
  review any direct reuse against project licensing requirements.

### [LibRawVNG0223]

- Source:
  [LibRaw 0.22.3 `vng_interpolate`](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L199)
- Supports: comparative VNG behavior and implementation differences.
- Licence: LibRaw is LGPL/CDDL. Review any direct reuse against project
  licensing requirements.

## Adding a reference

Record an authoritative stable URL or pinned commit, what behavior it supports,
known implementation differences, and the licence constraints that affect
reuse. Use an access date for a website. Verify that a DOI or URL resolves
before adding it.

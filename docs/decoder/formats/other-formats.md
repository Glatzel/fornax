# Other RAW formats policy

**Tier:** 3, may never

Everything not named in tiers 0 through 2 is outside the current product
scope. This includes CR2, CRW, ORF, PEF, DCR, KDC, MRW, X3F, and other RAW
containers even when Rawler or LibRaw can read them.

The decoder may recognize a container signature and return
`DecodeError::UnsupportedFormat` or `DecodeError::UnsupportedVariant` with the
container name. It must not fall through to a guessed TIFF interpretation or a
generic "backend failure".

Adding another format requires an explicit request, a new format plan, corpus
rows per camera, and a new work item. Do not widen support opportunistically
because an existing reference decoder handles the file.

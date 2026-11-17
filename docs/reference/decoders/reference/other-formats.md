# Other RAW format policy

This policy defines the unsupported RAW boundary and requirements for adding a
new format.

Everything outside tiers 0-2 is outside the current product scope, including
CR2, CRW, ORF, PEF, DCR, KDC, MRW, X3F, and other RAW containers even when a
reference decoder can read them.

The decoder may recognize a container signature and return
`DecodeError::UnsupportedFormat` or `DecodeError::UnsupportedVariant` with the
container name. It must not guess a TIFF interpretation or return a generic
backend failure.

Adding a format requires an explicit request, a new format plan, corpus rows
per camera, and a new work item. Support must not widen opportunistically
because another decoder handles the file.

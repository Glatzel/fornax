# Glossary

Terms are ordered alphabetically. Follow the linked source when a chapter needs
more detail than this working definition.

| Term | Working definition | Source |
| --- | --- | --- |
| Active area | The image region selected after decoder-specific framing is removed. | [Architecture](../../docs/architecture/overview.md) |
| Bayer filter array | A repeating 2x2 color filter layout with one red, two green, and one blue site. | [Bayer patterns](../../crates/fornax-raw-core/src/lib.rs) |
| Black level | The input code assigned to no light for a channel or sensor region. | [Preprocessor](../../crates/fornax-raw-pipeline/src/lib.rs) |
| CFA | Color filter array: the color-selective pattern placed over sensor sites. | [Architecture](../../docs/architecture/overview.md) |
| Demosaicing | Reconstruction of missing color samples from a color-filtered mosaic. | [Architecture](../../docs/architecture/overview.md) |
| Metadata | Decoder-supplied framing, levels, CFA pattern, and optional white balance. | [Architecture](../../docs/architecture/overview.md) |
| Mosaic | A single-channel image whose sites have different CFA colors. | [Architecture](../../docs/architecture/overview.md) |
| Normalization | Mapping black and white input levels to a common numeric range. | [Preprocessor](../../crates/fornax-raw-pipeline/src/lib.rs) |
| Pipeline stage | A typed processing boundary with its own input and output contract. | [Architecture](../../docs/architecture/overview.md) |
| Post-processing | RGB-domain work after demosaicing, including white balance in the current default pipeline. | [Architecture](../../docs/architecture/overview.md) |
| Preprocessing | Preparing an active decoder mosaic for demosaicing. | [Architecture](../../docs/architecture/overview.md) |
| RAW file | A camera-specific record containing sensor samples and metadata. | [Architecture](../../docs/architecture/overview.md) |
| White level | The input code that represents the upper end of a sensor's recorded range. | [Preprocessor](../../crates/fornax-raw-pipeline/src/lib.rs) |

When a glossary entry and a chapter disagree, verify the behavior in the cited
code and update this file with the chapter that changes the convention.

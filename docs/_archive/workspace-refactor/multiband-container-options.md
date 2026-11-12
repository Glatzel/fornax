# Multi-band container options

This note records evidence for the reserved multi-band container decision. It
compares two implementation directions without defining a container shape or
selecting an option.

## Current single-band behavior

`SampleBuffer` is a `#[non_exhaustive]` enum with five public variants:

| Variant | Sample type | Public payload | Raw extraction |
| --- | --- | --- | --- |
| `U8` | `u8` | `image::ImageBuffer<Luma<u8>, Vec<u8>>` | `Option<Vec<u8>>` |
| `U16` | `u16` | `image::ImageBuffer<Luma<u16>, Vec<u16>>` | `Option<Vec<u16>>` |
| `U32` | `u32` | `image::ImageBuffer<Luma<u32>, Vec<u32>>` | `Option<Vec<u32>>` |
| `F32` | `f32` | `image::ImageBuffer<Luma<f32>, Vec<f32>>` | `Option<Vec<f32>>` |
| `F64` | `f64` | `image::ImageBuffer<Luma<f64>, Vec<f64>>` | `Option<Vec<f64>>` |

The payloads are all mono images. `Luma` stores one scalar channel, and the
codec documentation explicitly says those samples are not necessarily
luminance. Construction checks the `u32` dimensions, multiplies them with
checked `usize` arithmetic, requires exactly one sample per pixel, and then
calls `ImageBuffer::from_raw`. `width` and `height` return `u32`; each
type-specific extraction consumes the enum and returns its flat sample vector
without copying when the variant matches.

The workspace uses a git-sourced `image` 0.25.9 release through the repository
dependency fork. The lockfile also contains an `image` 0.25.10 registry
package for other dependency paths. Because the `SampleBuffer` variant payloads
are public type aliases for this 0.x type, an `image` update is a public API
concern even where the sample scalar is unchanged.

The rest of the current pipeline assumes one plane:

- `DecodeRequest` has image and chunk dimensions, sample width and format,
  byte order, chunks, and a predictor. It has no band count or planar-layout
  field. `EncodeRequest` similarly starts from one `SampleBuffer`.
- Predictor layout is one scalar row with
  `width * bytes_per_sample` bytes. Horizontal integer prediction carries
  across scalar samples within that row. Floating-point prediction shuffles
  byte planes by the same scalar sample count.
- `RasterLayout` reads `ImageWidth`, `ImageLength`, strip or tile dimensions,
  and one row-major offset/byte-count pair per geometric chunk. Strips use the
  full image width; tiles use `TileWidth` and `TileLength`. It does not read
  `SamplesPerPixel` or `PlanarConfiguration`, and its expected chunk count is
  only the strip/tile grid.
- DNG decode accepts CFA and `LinearRaw` as raw photometric values, then rejects
  any `SamplesPerPixel` other than one before making a raster request.
  Multi-channel `LinearRaw` therefore reaches an explicit unsupported path.
- The private DNG adapter converts each of the four `RawMosaic` types to the
  matching `SampleBuffer` mono variant. The reverse conversion accepts U8,
  U16, U32, and F32 and rejects F64 and any other layout.
- DNG writing converts `RawMosaic` through that adapter and emits mono CFA
  metadata. Its structural tags set `SamplesPerPixel` and
  `PlanarConfiguration` to one.

Signed integer handling is not present in the current codec path. TIFF
`SampleFormat` value 2 is not mapped by the DNG decoder, `SampleFormat` in the
codec distinguishes only integer and float, and integer unpacking stores
unsigned values. F64 is present in the codec and its request/encode paths, but
the raw mosaic boundary does not carry it, and the raw pipeline's public
`Primitive` trait supporting `f64` does not imply `RawMosaic` or DNG adapter
support.

## Option: additive `SampleBuffer` variants

This option extends the existing enum and keeps one public result type for
codec output. The note does not decide which variants are added, what each
contains, or how bands are stored.

### Source compatibility and exhaustive matching

The enum is already non-exhaustive, so downstream exhaustive matching must
already contain a fallback. Adding a variant is mainly source-compatible for
downstream matching by itself. Downstream utility code can still compile while
taking a fallback for the new sample class, which may turn an implementation
gap into a runtime rejection rather than a compile error.

Within `fornax-codec` and `fornax-dng`, the existing matches over variants are
currently exhaustive. New variants require deliberate updates in dimension
access, raw extraction, encoding metadata, byte serialization, codec-specific
support, and DNG adaptation. Missing one internal arm is a compile failure;
handling it with a broad unsupported fallback is a behavioral choice.

Existing callers that only construct or match existing mono variants retain
their current payload types unless the enum's existing variants are changed.
New construction helpers and accessors would enlarge the public surface, and
the semantics of a wildcard match would broaden.

### Width, height, and raw access

The width and height contract must distinguish pixels from scalar sample
columns if a variant carries multiple bands. Existing mono callers expect
one scalar per pixel. A multiband variant also needs a way to describe or
recover its band structure; a flat `Vec<T>` alone does not express whether
data is interleaved by pixel, split by plane, or grouped in another order.

Existing per-type extraction helpers can remain unchanged, but they do not
provide a general extraction route for newly added variants. Matching over the
enum would be the compatibility boundary for every consumer that needs raw
samples.

### DecodeRequest and EncodeRequest

Adding a variant to the output enum does not give decode enough information to
locate and assemble multiple bands. The request would still need source/frame
metadata that describes the encoded organization, or the request would need a
higher-level input representation. Otherwise chunks can be decompressed but
cannot be placed into a correctly shaped multiband result.

Encode currently derives sample type and bits from the buffer variant.
Additional variants would extend that dispatch, but chunk dimensions and
requests still communicate only one geometric grid. Band organization and
row-sample count must either be encoded in those request fields' meaning or
carried separately.

### Predictor geometry

Predictors receive scalar width and height. They do not receive a band count or
planar organization. If a new request dimension counts pixels rather than
scalar samples, multiband row size and byte-plane sizes are underspecified.
If it counts scalar samples, those dimensions no longer directly match the
documented image width and height. Horizontal prediction also has different
row boundaries under chunky and planar storage.

Additive variants can coexist with the current predictor functions only after
the request-to-predictor geometry supplies enough information to calculate the
encoded row and reset boundaries.

### TIFF strip and tile layout

The current raster layer validates strip/tile locations but not sample planes.
An additive enum variant does not resolve whether a strip or tile contains
multiple bands, whether each band has separate chunks, or how a partial edge
tile accounts for all bands. Those are properties of the TIFF tags and encoded
payload, not properties of the decoded Rust sample variant alone.

The TIFF adapter must continue to own tag interpretation and must not leak
format-specific types into `fornax-codec`. An additive codec result may be a
natural destination for assembled samples, but it does not replace the
missing band-aware request/layout contract.

### DNG adaptation and `LinearRaw`

Each added variant needs an explicit DNG boundary outcome. The decoder returns
`RawMosaic`, so a multi-band codec result either needs a supported lossless
adaptation into the raw workflow or a deliberate unsupported error. The
existing adapter's fallback already treats layouts outside the raw mosaic enum
as unsupported, but broadening that fallback could obscure a new variant's
intended behavior.

Mono CFA and mono `LinearRaw` can continue using the current one-plane path.
Supporting multi-channel `LinearRaw` requires changing the
`SamplesPerPixel != 1` gate and defining the raw-workflow result, not merely
adding a buffer variant. Keeping that case unsupported is a separate product
decision and is not made here.

### Raw-workflow separation

Variants that only describe sample storage can remain format-neutral and keep
TIFF/DNG/raw metadata in their owning crates. If additivity starts to model
CFA meaning, DNG planes, lenses, or processing stages, it crosses the codec
boundary. The existing separation remains viable only while the enum stays a
codec-result description and conversions stay in format or workflow crates.

### `image` 0.x public-type compatibility

This option retains the current public exposure of
`ImageBuffer<Luma<T>, Vec<T>>`. Existing mono callers see no type change.
Additional variants could expose the same `image` dependency for their payload
and therefore inherit the same 0.x compatibility risk. Using a multi-channel
`image` pixel type would also make channel typing part of the public contract;
whether that is acceptable is unresolved.

Keeping the enum public also keeps constructors, variant payloads, pattern
matching, and any future `image` upgrade coupled. A compatibility layer can
reduce this exposure only if the existing variants or entry points are also
changed, which introduces source compatibility work of its own.

### i16 and f64 implications

No i16 storage or signed-integer wire interpretation exists today. An additive
variant for signed samples would need matches in decode, encode, raw
extraction, TIFF/DNG sample-format translation, predictor validation, and raw
mosaic adaptation. It would also require a signedness contract in codec
requests; adding an enum variant alone does not decode TIFF `SampleFormat` 2.

F64 already exists as a mono codec variant, so treating "wider float" as a new
container problem would mix two concerns. Its current support is bounded by
codec operations and by the DNG/raw boundaries, which reject it. Any additive
multiband arrangement containing f64 expands memory and serialization size
again. Preserving bit patterns, byte order, predictor behavior, and whether
f64 reaches the raw workflow are independent questions.

## Option: a separate multi-band/raster container

This option leaves the mono `SampleBuffer` path in place and introduces a
separate container for multi-band/raster samples used between codec and format
adapters. The note does not decide the type, ownership, crate placement,
lifetimes, bounds, or mono/multiband conversion policy.

### Source compatibility and exhaustive matching

Keeping the existing enum untouched avoids new-variant churn in downstream
matches over `SampleBuffer`. The compatibility risk moves to codec entry
points and adapters: if an existing function must accept or return a different
type, callers change; if mono and multiband paths are separate, existing mono
callers can remain while new callers choose the other path.

The separation introduces a dispatch question at every boundary: which
representation enters a codec, which one comes out, and whether mono data is
converted or keeps its current fast path. A new public container also creates
its own exhaustive-match and feature-evolution pressure, even though it does
not perturb the old enum.

### Width, height, and raw access

A separate container can expose dimensions and band organization together
without overloading the mono enum's dimension contract. It can centralize raw
sample access and validate the relationship between pixel dimensions, band
count, and storage organization.

The cost is either duplicated dimension/access logic or a conversion layer
between the mono and multiband representations. If mono calls are routed
through the new container, source/API compatibility and conversion overhead
must be evaluated. If mono remains independent, two paths can diverge in
validation and edge-case behavior.

### DecodeRequest and EncodeRequest

The codec can gain a dedicated request/result path without changing the
meaning of the current single-plane request fields, provided existing mono
entry points remain. Alternatively, the new container can be passed through
the current request shape through an adapter. Those are different public API
consequences and cannot be inferred from the container alone.

The request must still carry or derive encoded band organization. A separate
container at the result boundary cannot recover information discarded while
planning strip/tile decoding. Format crates remain responsible for resolving
compression IDs and TIFF/DNG tag semantics before handing neutral geometry to
the codec.

### Predictor geometry

A container-level geometry model can make pixel extent, sample multiplicity,
chunk extent, and storage organization explicit to predictor setup. That can
avoid overloading the current scalar width. It also requires the codec to
translate that model into the existing predictor representation or maintain a
parallel predictor interface.

The same TIFF semantic questions remain: row length, component interleaving,
plane reset rules, byte-plane stride, and edge-tile sizing. A separate
container makes room for those distinctions; it does not decide their
semantics or guarantee that every codec supports all organizations.

### TIFF strip and tile layout

A TIFF adapter can map `SamplesPerPixel`, byte counts, strip/tile geometry,
and planar organization into the neutral container while keeping the TIFF
types in `fornax-tiff`. This option draws a clear boundary around those tags.

It does not remove the need to decide whether the container preserves the
encoded organization or stores one canonical organization. Preserving it puts
more metadata and conversion complexity in the container path. Canonicalizing
it may lose information needed by codecs or writers and requires explicit
conversion rules for strips, tiles, partial edges, and separated planes.

### DNG adaptation and `LinearRaw`

A separate container permits mono and multiband DNG results to have distinct
adapter paths. Mono can continue converting to `RawMosaic`; multiband
`LinearRaw` can be preserved outside the raw mosaic workflow or rejected with
a targeted unsupported result. The choice of which outcome is supported
remains open.

If the adapter grows into a general conversion layer, it must define when
bands are collapsed, rejected, or transformed. That layer could prevent
TIFF/DNG-specific semantics from entering `fornax-codec`, but it also becomes
a new owner of cross-crate compatibility policy. Without a conversion layer,
callers face separate mono and multiband APIs and must choose explicitly.

### Raw-workflow separation

The container can stay neutral while format-specific interpretation remains in
TIFF/DNG adapters and raw-stage meaning remains in the raw workflow crates.
This preserves the rule that `fornax-codec` contains no TIFF or raw-workflow
types.

The dependency direction still matters. If the codec must depend on a
workflow-owned container, the codec stops being independently usable by
format crates. If format crates depend on a codec-owned neutral container,
the container must avoid format tags and process semantics. If the container
lives in another neutral crate, crate naming and placement become part of the
reserved workspace decision rather than a documentation detail.

### `image` 0.x public-type compatibility

A new container can avoid exposing `image` types for its own payload and can
provide mono conversion without replacing the existing public enum. That
reduces new exposure to `image` 0.x changes.

The existing mono `SampleBuffer` variants still expose `image` types unless a
separate compatibility decision changes them. A conversion path therefore
does not by itself remove the dependency or the risk. It may also add a copy
when converting between an image-backed mono buffer and the new container,
unless sharing or borrowing behavior is explicitly designed.

### i16 and f64 implications

A type-neutral container can defer storage representation and carry signed
integer/wide-float format information separately from the pure sample vector.
That avoids treating each numeric type as a new mono enum variant. It does not
define whether signed samples are represented at the container boundary,
whether f64 is preserved bit-for-bit, or which codecs and adapters support it.

Any representation still affects chunk byte counts, endian conversion,
predictor byte planes, overflow checks, memory use, and DNG/raw adaptation.
In particular, an f64 multiband payload can multiply memory and encoded size,
while i16 requires signed wire decoding that the current integer path does not
provide. Mono F64 support in `SampleBuffer` remains a separate existing
capability from raw-workflow F64 support.

## Comparison

| Question | Additive variants | Separate container | Evidence |
| --- | --- | --- | --- |
| What happens to existing source code and exhaustive matches? | Existing mono payloads remain, and non-exhaustive downstream matches should tolerate new variants, but internal codec/DNG matches require updates and wildcard fallbacks can hide missing support. | Existing `SampleBuffer` matches remain stable, but codec/request entry points, adapters, or dispatch between mono and multiband paths may change. | [sample_buffer.rs](../../../crates/fornax-codec/src/sample_buffer.rs#L21), [encode.rs](../../../crates/fornax-codec/src/encode.rs#L13), [DNG adapter](../../../crates/fornax-dng/src/sample_buffer.rs#L5) |
| How are width, height, and raw samples obtained? | Existing mono accessors stay; multiband dimensions and raw extraction broaden the enum contract and need explicit pixel-versus-sample and band-organization semantics. | Dimensions, bands, and raw access can be grouped, but need either conversion from the image-backed mono path or duplicated validation/access logic. | [sample_buffer.rs](../../../crates/fornax-codec/src/sample_buffer.rs#L149), [primitive.rs](../../../crates/fornax-raw-core/src/primitive.rs#L19) |
| What must change in decode and encode requests? | A new result variant does not supply missing band metadata; request dimensions and storage organization must become sufficient for assembly. | A separate request/result path can preserve mono APIs, but public compatibility depends on whether existing entry points are retained or adapted. | [request.rs](../../../crates/fornax-codec/src/request.rs#L146), [encode.rs](../../../crates/fornax-codec/src/encode.rs#L13) |
| How does predictor geometry account for bands? | Predictor layout still needs band organization and row-sample count, even though the result can be represented by a new variant. | Geometry can be modeled explicitly but must bridge to predictor logic shared with mono without divergent reset semantics. | [predictor.rs](../../../crates/fornax-codec/src/predictor.rs#L34), [samples.rs](../../../crates/fornax-codec/src/samples.rs#L8) |
| What TIFF strip/tile work is required? | The enum does not interpret `SamplesPerPixel` or `PlanarConfiguration`; format adapters and request geometry still need band-aware chunks and edge handling. | A neutral adapter/container boundary can carry the layout, but preservation versus canonicalization remains an open design choice. | [raster.rs](../../../crates/fornax-tiff/src/raster.rs#L92), [raster request](../../../crates/fornax-tiff/src/raster.rs#L562) |
| What happens to mono DNG and multi-channel `LinearRaw`? | Every new result needs an adapter outcome; multi-channel `LinearRaw` still requires removing the current samples-per-pixel gate and defining raw-workflow handling. | Mono can keep the `RawMosaic` path while multiband has a distinct preserve-or-reject adapter, but conversions may grow a new policy layer. | [DNG decode plan](../../../crates/fornax-dng/src/dng_decoder/mod.rs#L155), [DNG adapter](../../../crates/fornax-dng/src/sample_buffer.rs#L14) |
| How is raw-workflow separation preserved? | Viable while variants remain format-neutral codec results; format and raw metadata stay in their current owners. | Viable if dependency direction keeps TIFF/raw types outside the codec, but placement and conversion ownership become explicit decisions. | [codec crate docs](../../../crates/fornax-codec/src/lib.rs#L1), [primitive.rs](../../../crates/fornax-raw-core/src/primitive.rs#L24) |
| How compatible is the public `image` 0.x type exposure? | Existing and new mono variants can keep exposing the same `ImageBuffer<Luma<T>, Vec<T>>`; `image` upgrades remain visible in the public enum. | The new container can avoid that exposure, but the old mono variants remain image-backed and conversions may add copies. | [sample_buffer.rs](../../../crates/fornax-codec/src/sample_buffer.rs#L4), [Cargo.lock](../../../Cargo.lock#L1171) |
| How do i16 and f64 affect storage? | Signed samples need new enum behavior and request semantics; mono f64 stays available but bounded by codec and raw/DNG support. | Format metadata can avoid one variant per width, but representation, bit preservation, codec support, and mono/raw conversions remain undecided. | [request sample format](../../../crates/fornax-codec/src/request.rs#L82), [DNG sample format mapping](../../../crates/fornax-dng/src/dng_decoder/mod.rs#L172), [primitive.rs](../../../crates/fornax-raw-core/src/primitive.rs#L8) |

## Unresolved questions

- Is the target a codec-level sample container, a TIFF raster representation,
  or a raw-workflow image, and which layer owns band semantics?
- Do image dimensions continue to count pixels while requests count scalar
  samples, or do those meanings need to diverge explicitly?
- Must strips/tiles and chunky/planar organizations be preserved, or can an
  adapter canonicalize them before codec assembly?
- Should mono remain on its existing path, be adapted into a general path, or
  use both depending on the caller?
- Are existing `SampleBuffer` variants and their image-backed payloads part of
  the compatibility promise across `image` 0.x updates?
- How should TIFF signed integer samples be represented and which codecs,
  predictors, and raw stages must accept them?
- Is F64 required only at the codec boundary, or must it survive DNG/raw
  adaptation without narrowing or converting the raw pipeline?
- Does multi-channel `LinearRaw` remain unsupported, and if not, which stage
  owns preserving or interpreting its bands?
- Where can neutral band metadata live without making `fornax-codec` depend on
  TIFF or raw-workflow types?
- What downstream source-compatibility guarantees apply to codec request
  fields, result types, and conversions?

These questions sit inside the
[Phase B-C reserved decisions](phase-b-c-handoff.md#reserved-decisions).

## Options and consequences

The additive option keeps one result enum and preserves the existing mono
payloads, at the cost of expanding every internal variant match and
continuing to expose `image` types through the codec API. It leaves band
organization to request fields, accessors, or external conventions unless the
enum itself is made more semantic.

The separate-container option can isolate multiband dimensions, layout, and
sample access from the mono enum and can keep format-specific metadata in
adapter crates. Its consequences are parallel or converted mono/multiband
paths, a new public compatibility surface, and a need to choose whether layout
is preserved or canonicalized.

Neither option defines signed-integer support, f64 workflow support, DNG
`LinearRaw` behavior, or TIFF band layout by itself.

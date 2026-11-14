CREATE TEMP TABLE selected AS
SELECT
    path::VARCHAR AS path
FROM read_csv('@SELECTED@', header = true, all_varchar = true);

CREATE TEMP TABLE baseline AS
SELECT
    baseline.*
FROM read_csv('@BASELINE@', header = true, all_varchar = true) AS baseline
SEMI JOIN selected USING (path);

CREATE TEMP TABLE candidate AS
SELECT
    candidate.status::VARCHAR AS status,
    candidate.camera::VARCHAR AS camera,
    candidate.format::VARCHAR AS format,
    candidate.path::VARCHAR AS path,
    candidate.decode_pass::INTEGER AS decode_pass,
    candidate.raw_mosaic_pass::INTEGER AS raw_mosaic_pass,
    candidate.raw_sha256::VARCHAR AS raw_sha256,
    candidate.process_metadata_pass::INTEGER AS process_metadata_pass,
    candidate.handoff_pass::INTEGER AS handoff_pass,
    candidate.error_kind::VARCHAR AS error_kind,
    candidate.width::INTEGER AS width,
    candidate.height::INTEGER AS height,
    candidate.cfa::VARCHAR AS cfa,
    candidate.compression_id::INTEGER AS compression_id,
    candidate.compression_name::VARCHAR AS compression_name,
    candidate.compression_lossless::INTEGER AS compression_lossless,
    candidate.storage::VARCHAR AS storage,
    candidate.bits_per_sample::INTEGER AS bits_per_sample,
    candidate.sample_format::VARCHAR AS sample_format,
    candidate.predictor::INTEGER AS predictor,
    candidate.rows_per_strip::INTEGER AS rows_per_strip,
    candidate.tile_width::INTEGER AS tile_width,
    candidate.tile_height::INTEGER AS tile_height
FROM read_csv('@CANDIDATE@', header = true, all_varchar = true) AS candidate;

CREATE TEMP TABLE diff_rows AS
SELECT
    'missing'::VARCHAR AS classification,
    baseline.camera,
    baseline.format,
    baseline.path,
    baseline.decode_pass::INTEGER AS before_decode_pass,
    baseline.raw_mosaic_pass::INTEGER AS before_raw_mosaic_pass,
    baseline.raw_sha256 AS before_raw_sha256,
    baseline.process_metadata_pass::INTEGER AS before_process_metadata_pass,
    baseline.handoff_pass::INTEGER AS before_handoff_pass,
    baseline.error_kind AS before_error_kind,
    baseline.width::INTEGER AS before_width,
    baseline.height::INTEGER AS before_height,
    baseline.cfa AS before_cfa,
    baseline.compression_id::INTEGER AS before_compression_id,
    baseline.compression_name AS before_compression_name,
    baseline.compression_lossless::INTEGER AS before_compression_lossless,
    baseline.storage AS before_storage,
    baseline.bits_per_sample::INTEGER AS before_bits_per_sample,
    baseline.sample_format AS before_sample_format,
    baseline.predictor::INTEGER AS before_predictor,
    baseline.rows_per_strip::INTEGER AS before_rows_per_strip,
    baseline.tile_width::INTEGER AS before_tile_width,
    baseline.tile_height::INTEGER AS before_tile_height,
    NULL::INTEGER AS after_decode_pass,
    NULL::INTEGER AS after_raw_mosaic_pass,
    NULL::VARCHAR AS after_raw_sha256,
    NULL::INTEGER AS after_process_metadata_pass,
    NULL::INTEGER AS after_handoff_pass,
    NULL::VARCHAR AS after_error_kind,
    NULL::INTEGER AS after_width,
    NULL::INTEGER AS after_height,
    NULL::VARCHAR AS after_cfa,
    NULL::INTEGER AS after_compression_id,
    NULL::VARCHAR AS after_compression_name,
    NULL::INTEGER AS after_compression_lossless,
    NULL::VARCHAR AS after_storage,
    NULL::INTEGER AS after_bits_per_sample,
    NULL::VARCHAR AS after_sample_format,
    NULL::INTEGER AS after_predictor,
    NULL::INTEGER AS after_rows_per_strip,
    NULL::INTEGER AS after_tile_width,
    NULL::INTEGER AS after_tile_height
FROM baseline
ANTI JOIN candidate USING (path)

UNION ALL

SELECT
    'extra'::VARCHAR AS classification,
    candidate.camera,
    candidate.format,
    candidate.path,
    NULL::INTEGER AS before_decode_pass,
    NULL::INTEGER AS before_raw_mosaic_pass,
    NULL::VARCHAR AS before_raw_sha256,
    NULL::INTEGER AS before_process_metadata_pass,
    NULL::INTEGER AS before_handoff_pass,
    NULL::VARCHAR AS before_error_kind,
    NULL::INTEGER AS before_width,
    NULL::INTEGER AS before_height,
    NULL::VARCHAR AS before_cfa,
    NULL::INTEGER AS before_compression_id,
    NULL::VARCHAR AS before_compression_name,
    NULL::INTEGER AS before_compression_lossless,
    NULL::VARCHAR AS before_storage,
    NULL::INTEGER AS before_bits_per_sample,
    NULL::VARCHAR AS before_sample_format,
    NULL::INTEGER AS before_predictor,
    NULL::INTEGER AS before_rows_per_strip,
    NULL::INTEGER AS before_tile_width,
    NULL::INTEGER AS before_tile_height,
    candidate.decode_pass AS after_decode_pass,
    candidate.raw_mosaic_pass AS after_raw_mosaic_pass,
    candidate.raw_sha256 AS after_raw_sha256,
    candidate.process_metadata_pass AS after_process_metadata_pass,
    candidate.handoff_pass AS after_handoff_pass,
    candidate.error_kind AS after_error_kind,
    candidate.width AS after_width,
    candidate.height AS after_height,
    candidate.cfa AS after_cfa,
    candidate.compression_id AS after_compression_id,
    candidate.compression_name AS after_compression_name,
    candidate.compression_lossless AS after_compression_lossless,
    candidate.storage AS after_storage,
    candidate.bits_per_sample AS after_bits_per_sample,
    candidate.sample_format AS after_sample_format,
    candidate.predictor AS after_predictor,
    candidate.rows_per_strip AS after_rows_per_strip,
    candidate.tile_width AS after_tile_width,
    candidate.tile_height AS after_tile_height
FROM candidate
ANTI JOIN baseline USING (path)

UNION ALL

SELECT
    'changed'::VARCHAR AS classification,
    baseline.camera,
    baseline.format,
    baseline.path,
    baseline.decode_pass::INTEGER AS before_decode_pass,
    baseline.raw_mosaic_pass::INTEGER AS before_raw_mosaic_pass,
    baseline.raw_sha256 AS before_raw_sha256,
    baseline.process_metadata_pass::INTEGER AS before_process_metadata_pass,
    baseline.handoff_pass::INTEGER AS before_handoff_pass,
    baseline.error_kind AS before_error_kind,
    baseline.width::INTEGER AS before_width,
    baseline.height::INTEGER AS before_height,
    baseline.cfa AS before_cfa,
    baseline.compression_id::INTEGER AS before_compression_id,
    baseline.compression_name AS before_compression_name,
    baseline.compression_lossless::INTEGER AS before_compression_lossless,
    baseline.storage AS before_storage,
    baseline.bits_per_sample::INTEGER AS before_bits_per_sample,
    baseline.sample_format AS before_sample_format,
    baseline.predictor::INTEGER AS before_predictor,
    baseline.rows_per_strip::INTEGER AS before_rows_per_strip,
    baseline.tile_width::INTEGER AS before_tile_width,
    baseline.tile_height::INTEGER AS before_tile_height,
    candidate.decode_pass AS after_decode_pass,
    candidate.raw_mosaic_pass AS after_raw_mosaic_pass,
    candidate.raw_sha256 AS after_raw_sha256,
    candidate.process_metadata_pass AS after_process_metadata_pass,
    candidate.handoff_pass AS after_handoff_pass,
    candidate.error_kind AS after_error_kind,
    candidate.width AS after_width,
    candidate.height AS after_height,
    candidate.cfa AS after_cfa,
    candidate.compression_id AS after_compression_id,
    candidate.compression_name AS after_compression_name,
    candidate.compression_lossless AS after_compression_lossless,
    candidate.storage AS after_storage,
    candidate.bits_per_sample AS after_bits_per_sample,
    candidate.sample_format AS after_sample_format,
    candidate.predictor AS after_predictor,
    candidate.rows_per_strip AS after_rows_per_strip,
    candidate.tile_width AS after_tile_width,
    candidate.tile_height AS after_tile_height
FROM baseline
JOIN candidate USING (path)
WHERE
    baseline.decode_pass::INTEGER IS DISTINCT FROM candidate.decode_pass
    OR baseline.raw_mosaic_pass::INTEGER IS DISTINCT FROM candidate.raw_mosaic_pass
    OR baseline.raw_sha256::VARCHAR IS DISTINCT FROM candidate.raw_sha256
    OR baseline.process_metadata_pass::INTEGER IS DISTINCT FROM candidate.process_metadata_pass
    OR baseline.handoff_pass::INTEGER IS DISTINCT FROM candidate.handoff_pass
    OR baseline.error_kind::VARCHAR IS DISTINCT FROM candidate.error_kind
    OR baseline.width::INTEGER IS DISTINCT FROM candidate.width
    OR baseline.height::INTEGER IS DISTINCT FROM candidate.height
    OR baseline.cfa::VARCHAR IS DISTINCT FROM candidate.cfa
    OR baseline.compression_id::INTEGER IS DISTINCT FROM candidate.compression_id
    OR baseline.compression_name::VARCHAR IS DISTINCT FROM candidate.compression_name
    OR baseline.compression_lossless::INTEGER IS DISTINCT FROM candidate.compression_lossless
    OR baseline.storage::VARCHAR IS DISTINCT FROM candidate.storage
    OR baseline.bits_per_sample::INTEGER IS DISTINCT FROM candidate.bits_per_sample
    OR baseline.sample_format::VARCHAR IS DISTINCT FROM candidate.sample_format
    OR baseline.predictor::INTEGER IS DISTINCT FROM candidate.predictor
    OR baseline.rows_per_strip::INTEGER IS DISTINCT FROM candidate.rows_per_strip
    OR baseline.tile_width::INTEGER IS DISTINCT FROM candidate.tile_width
    OR baseline.tile_height::INTEGER IS DISTINCT FROM candidate.tile_height
    OR baseline.status::VARCHAR IS DISTINCT FROM candidate.status;

COPY (
    SELECT *
    FROM diff_rows
    ORDER BY classification, camera, format, path
) TO '@OUTPUT@' (
    FORMAT CSV,
    HEADER,
    DELIMITER ',',
    QUOTE '"',
    NEW_LINE '\n'
);

SELECT
    count() FILTER (WHERE classification = 'missing') AS missing,
    count() FILTER (WHERE classification = 'extra') AS extra,
    count() FILTER (WHERE classification = 'changed') AS changed
FROM diff_rows;

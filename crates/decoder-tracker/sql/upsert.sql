CREATE TEMP TABLE baseline AS
SELECT
    baseline.status::VARCHAR AS status,
    baseline.camera::VARCHAR AS camera,
    baseline.format::VARCHAR AS format,
    baseline.path::VARCHAR AS path,
    baseline.decode_pass::INTEGER AS decode_pass,
    baseline.raw_mosaic_pass::INTEGER AS raw_mosaic_pass,
    baseline.raw_sha256::VARCHAR AS raw_sha256,
    baseline.process_metadata_pass::INTEGER AS process_metadata_pass,
    baseline.handoff_pass::INTEGER AS handoff_pass,
    baseline.error_kind::VARCHAR AS error_kind,
    baseline.width::INTEGER AS width,
    baseline.height::INTEGER AS height,
    baseline.cfa::VARCHAR AS cfa,
    baseline.compression_id::INTEGER AS compression_id,
    baseline.compression_name::VARCHAR AS compression_name,
    baseline.compression_lossless::INTEGER AS compression_lossless,
    baseline.storage::VARCHAR AS storage,
    baseline.bits_per_sample::INTEGER AS bits_per_sample,
    baseline.sample_format::VARCHAR AS sample_format,
    baseline.predictor::INTEGER AS predictor,
    baseline.rows_per_strip::INTEGER AS rows_per_strip,
    baseline.tile_width::INTEGER AS tile_width,
    baseline.tile_height::INTEGER AS tile_height
FROM read_csv('@BASELINE@', header = true, all_varchar = true) AS baseline;

CREATE TEMP TABLE updates AS
SELECT
    updates.status::VARCHAR AS status,
    updates.camera::VARCHAR AS camera,
    updates.format::VARCHAR AS format,
    updates.path::VARCHAR AS path,
    updates.decode_pass::INTEGER AS decode_pass,
    updates.raw_mosaic_pass::INTEGER AS raw_mosaic_pass,
    updates.raw_sha256::VARCHAR AS raw_sha256,
    updates.process_metadata_pass::INTEGER AS process_metadata_pass,
    updates.handoff_pass::INTEGER AS handoff_pass,
    updates.error_kind::VARCHAR AS error_kind,
    updates.width::INTEGER AS width,
    updates.height::INTEGER AS height,
    updates.cfa::VARCHAR AS cfa,
    updates.compression_id::INTEGER AS compression_id,
    updates.compression_name::VARCHAR AS compression_name,
    updates.compression_lossless::INTEGER AS compression_lossless,
    updates.storage::VARCHAR AS storage,
    updates.bits_per_sample::INTEGER AS bits_per_sample,
    updates.sample_format::VARCHAR AS sample_format,
    updates.predictor::INTEGER AS predictor,
    updates.rows_per_strip::INTEGER AS rows_per_strip,
    updates.tile_width::INTEGER AS tile_width,
    updates.tile_height::INTEGER AS tile_height
FROM read_csv('@CANDIDATE@', header = true, all_varchar = true) AS updates;

CREATE TEMP TABLE merged AS
SELECT *
FROM (
    SELECT
        status,
        camera,
        format,
        path,
        decode_pass,
        raw_mosaic_pass,
        raw_sha256,
        process_metadata_pass,
        handoff_pass,
        error_kind,
        width,
        height,
        cfa,
        compression_id,
        compression_name,
        compression_lossless,
        storage,
        bits_per_sample,
        sample_format,
        predictor,
        rows_per_strip,
        tile_width,
        tile_height,
        0 AS source_priority
    FROM baseline
    UNION ALL
    SELECT
        status,
        camera,
        format,
        path,
        decode_pass,
        raw_mosaic_pass,
        raw_sha256,
        process_metadata_pass,
        handoff_pass,
        error_kind,
        width,
        height,
        cfa,
        compression_id,
        compression_name,
        compression_lossless,
        storage,
        bits_per_sample,
        sample_format,
        predictor,
        rows_per_strip,
        tile_width,
        tile_height,
        1 AS source_priority
    FROM updates
)
QUALIFY row_number() OVER (
    PARTITION BY camera, format, path
    ORDER BY source_priority DESC
) = 1;

COPY (
    SELECT
        status,
        camera,
        format,
        path,
        decode_pass,
        raw_mosaic_pass,
        raw_sha256,
        process_metadata_pass,
        handoff_pass,
        error_kind,
        width,
        height,
        cfa,
        compression_id,
        compression_name,
        compression_lossless,
        storage,
        bits_per_sample,
        sample_format,
        predictor,
        rows_per_strip,
        tile_width,
        tile_height
    FROM merged
    ORDER BY camera, format, path
) TO '@OUTPUT@' (
    FORMAT CSV,
    HEADER,
    DELIMITER ',',
    QUOTE '"',
    NEW_LINE '\n'
);

SELECT count() AS rows_merged FROM merged;

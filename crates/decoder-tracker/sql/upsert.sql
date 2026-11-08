CREATE TEMP TABLE baseline AS
SELECT
    baseline.vendor::VARCHAR AS vendor,
    baseline.camera::VARCHAR AS camera,
    baseline.format::VARCHAR AS format,
    baseline.path::VARCHAR AS path,
    baseline.decode_pass::INTEGER AS decode_pass,
    baseline.raw_mosaic_pass::INTEGER AS raw_mosaic_pass,
    baseline.process_metadata_pass::INTEGER AS process_metadata_pass,
    baseline.handoff_pass::INTEGER AS handoff_pass,
    baseline.error_kind::VARCHAR AS error_kind,
    baseline.width::INTEGER AS width,
    baseline.height::INTEGER AS height,
    baseline.cfa::VARCHAR AS cfa,
    baseline.status::VARCHAR AS status
FROM read_csv('@BASELINE@', header = true, all_varchar = true) AS baseline;

CREATE TEMP TABLE updates AS
SELECT
    updates.vendor::VARCHAR AS vendor,
    updates.camera::VARCHAR AS camera,
    updates.format::VARCHAR AS format,
    updates.path::VARCHAR AS path,
    updates.decode_pass::INTEGER AS decode_pass,
    updates.raw_mosaic_pass::INTEGER AS raw_mosaic_pass,
    updates.process_metadata_pass::INTEGER AS process_metadata_pass,
    updates.handoff_pass::INTEGER AS handoff_pass,
    updates.error_kind::VARCHAR AS error_kind,
    updates.width::INTEGER AS width,
    updates.height::INTEGER AS height,
    updates.cfa::VARCHAR AS cfa,
    updates.status::VARCHAR AS status
FROM read_csv('@CANDIDATE@', header = true, all_varchar = true) AS updates;

CREATE TEMP TABLE merged AS
SELECT *
FROM (
    SELECT
        vendor,
        camera,
        format,
        path,
        decode_pass,
        raw_mosaic_pass,
        process_metadata_pass,
        handoff_pass,
        error_kind,
        width,
        height,
        cfa,
        status,
        0 AS source_priority
    FROM baseline
    UNION ALL
    SELECT
        vendor,
        camera,
        format,
        path,
        decode_pass,
        raw_mosaic_pass,
        process_metadata_pass,
        handoff_pass,
        error_kind,
        width,
        height,
        cfa,
        status,
        1 AS source_priority
    FROM updates
)
QUALIFY row_number() OVER (
    PARTITION BY vendor, camera, format, path
    ORDER BY source_priority DESC
) = 1;

COPY (
    SELECT
        status,
        vendor,
        camera,
        format,
        path,
        decode_pass,
        raw_mosaic_pass,
        process_metadata_pass,
        handoff_pass,
        error_kind,
        width,
        height,
        cfa
    FROM merged
    ORDER BY vendor, camera, format, path
) TO '@OUTPUT@' (
    FORMAT CSV,
    HEADER,
    DELIMITER ',',
    QUOTE '"',
    NEW_LINE '\n'
);

SELECT count() AS rows_merged FROM merged;

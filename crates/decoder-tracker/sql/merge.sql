CREATE TEMP TABLE expected AS
SELECT
    vendor::VARCHAR AS vendor,
    camera::VARCHAR AS camera,
    format::VARCHAR AS format,
    path::VARCHAR AS path
FROM read_csv('@MANIFEST@', header = true, all_varchar = true);

CREATE TEMP TABLE updates AS
SELECT
    vendor::VARCHAR AS vendor,
    camera::VARCHAR AS camera,
    format::VARCHAR AS format,
    path::VARCHAR AS path,
    decode_pass::INTEGER AS decode_pass,
    raw_mosaic_pass::INTEGER AS raw_mosaic_pass,
    process_metadata_pass::INTEGER AS process_metadata_pass,
    handoff_pass::INTEGER AS handoff_pass,
    error_kind::VARCHAR AS error_kind,
    width::INTEGER AS width,
    height::INTEGER AS height,
    cfa::VARCHAR AS cfa,
    status::VARCHAR AS status
FROM read_csv([@SHARDS@], header = true, all_varchar = true, union_by_name = true);

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
    FROM updates
    ORDER BY vendor, camera, format, path
) TO '@OUTPUT@' (
    FORMAT CSV,
    HEADER,
    DELIMITER ',',
    QUOTE '"',
    NEW_LINE '\n'
);

SELECT
    (
        SELECT count()
        FROM (
            SELECT vendor, camera, format, path
            FROM updates
            GROUP BY ALL
            HAVING count() > 1
        )
    ) AS duplicates,
    (
        SELECT count()
        FROM (
            SELECT vendor, camera, format, path FROM expected
            EXCEPT
            SELECT vendor, camera, format, path FROM updates
        )
    ) AS missing,
    (
        SELECT count()
        FROM (
            SELECT vendor, camera, format, path FROM updates
            EXCEPT
            SELECT vendor, camera, format, path FROM expected
        )
    ) AS unexpected,
    (SELECT count() FROM updates) AS rows_merged;

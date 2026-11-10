CREATE TEMP TABLE expected AS
SELECT
    path::VARCHAR AS path
FROM read_csv('@MANIFEST@', header = true, all_varchar = true);

CREATE TEMP TABLE updates AS
SELECT
    status::VARCHAR AS status,
    camera::VARCHAR AS camera,
    format::VARCHAR AS format,
    path::VARCHAR AS path,
    decode_pass::INTEGER AS decode_pass,
    raw_mosaic_pass::INTEGER AS raw_mosaic_pass,
    raw_sha256::VARCHAR AS raw_sha256,
    process_metadata_pass::INTEGER AS process_metadata_pass,
    handoff_pass::INTEGER AS handoff_pass,
    error_kind::VARCHAR AS error_kind,
    width::INTEGER AS width,
    height::INTEGER AS height,
    cfa::VARCHAR AS cfa
FROM read_csv([@SHARDS@], header = true, all_varchar = true, union_by_name = true);

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
        cfa
    FROM updates
    ORDER BY camera, format, path
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
            SELECT path
            FROM updates
            GROUP BY ALL
            HAVING count() > 1
        )
    ) AS duplicates,
    (
        SELECT count()
        FROM (
            SELECT path FROM expected
            EXCEPT
            SELECT path FROM updates
        )
    ) AS missing,
    (
        SELECT count()
        FROM (
            SELECT path FROM updates
            EXCEPT
            SELECT path FROM expected
        )
    ) AS unexpected,
    (SELECT count() FROM updates) AS rows_merged,
    (
        SELECT count()
        FROM updates
        WHERE NOT (
            decode_pass IN (0, 1)
            AND raw_mosaic_pass IN (0, 1)
            AND process_metadata_pass IN (0, 1)
            AND handoff_pass IN (0, 1)
        )
    ) AS invalid_stage_values,
    (
        SELECT count()
        FROM updates
        WHERE
            (decode_pass = 0 AND (raw_mosaic_pass = 1 OR process_metadata_pass = 1 OR handoff_pass = 1))
            OR (raw_mosaic_pass = 0 AND (process_metadata_pass = 1 OR handoff_pass = 1))
            OR (process_metadata_pass = 0 AND handoff_pass = 1)
    ) AS invalid_stage_order,
    (
        SELECT count()
        FROM updates
        WHERE
            (
                decode_pass = 1
                AND raw_mosaic_pass = 1
                AND process_metadata_pass = 1
                AND handoff_pass = 1
            ) != (coalesce(error_kind, '') = '')
    ) AS invalid_error_kind,
    (
        SELECT count()
        FROM updates
        WHERE
            status != CASE
                WHEN decode_pass = 1
                    AND raw_mosaic_pass = 1
                    AND process_metadata_pass = 1
                    AND handoff_pass = 1
                THEN '✅'
                ELSE '❌'
            END
    ) AS invalid_status;

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
    candidate.process_metadata_pass::INTEGER AS process_metadata_pass,
    candidate.handoff_pass::INTEGER AS handoff_pass,
    candidate.error_kind::VARCHAR AS error_kind,
    candidate.width::INTEGER AS width,
    candidate.height::INTEGER AS height,
    candidate.cfa::VARCHAR AS cfa
FROM read_csv('@CANDIDATE@', header = true, all_varchar = true) AS candidate;

CREATE TEMP TABLE diff_rows AS
SELECT
    'missing'::VARCHAR AS classification,
    baseline.camera,
    baseline.format,
    baseline.path,
    baseline.decode_pass::INTEGER AS before_decode_pass,
    baseline.raw_mosaic_pass::INTEGER AS before_raw_mosaic_pass,
    baseline.process_metadata_pass::INTEGER AS before_process_metadata_pass,
    baseline.handoff_pass::INTEGER AS before_handoff_pass,
    baseline.error_kind AS before_error_kind,
    baseline.width::INTEGER AS before_width,
    baseline.height::INTEGER AS before_height,
    baseline.cfa AS before_cfa,
    NULL::INTEGER AS after_decode_pass,
    NULL::INTEGER AS after_raw_mosaic_pass,
    NULL::INTEGER AS after_process_metadata_pass,
    NULL::INTEGER AS after_handoff_pass,
    NULL::VARCHAR AS after_error_kind,
    NULL::INTEGER AS after_width,
    NULL::INTEGER AS after_height,
    NULL::VARCHAR AS after_cfa
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
    NULL::INTEGER AS before_process_metadata_pass,
    NULL::INTEGER AS before_handoff_pass,
    NULL::VARCHAR AS before_error_kind,
    NULL::INTEGER AS before_width,
    NULL::INTEGER AS before_height,
    NULL::VARCHAR AS before_cfa,
    candidate.decode_pass AS after_decode_pass,
    candidate.raw_mosaic_pass AS after_raw_mosaic_pass,
    candidate.process_metadata_pass AS after_process_metadata_pass,
    candidate.handoff_pass AS after_handoff_pass,
    candidate.error_kind AS after_error_kind,
    candidate.width AS after_width,
    candidate.height AS after_height,
    candidate.cfa AS after_cfa
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
    baseline.process_metadata_pass::INTEGER AS before_process_metadata_pass,
    baseline.handoff_pass::INTEGER AS before_handoff_pass,
    baseline.error_kind AS before_error_kind,
    baseline.width::INTEGER AS before_width,
    baseline.height::INTEGER AS before_height,
    baseline.cfa AS before_cfa,
    candidate.decode_pass AS after_decode_pass,
    candidate.raw_mosaic_pass AS after_raw_mosaic_pass,
    candidate.process_metadata_pass AS after_process_metadata_pass,
    candidate.handoff_pass AS after_handoff_pass,
    candidate.error_kind AS after_error_kind,
    candidate.width AS after_width,
    candidate.height AS after_height,
    candidate.cfa AS after_cfa
FROM baseline
JOIN candidate USING (path)
WHERE
    baseline.decode_pass::INTEGER IS DISTINCT FROM candidate.decode_pass
    OR baseline.raw_mosaic_pass::INTEGER IS DISTINCT FROM candidate.raw_mosaic_pass
    OR baseline.process_metadata_pass::INTEGER IS DISTINCT FROM candidate.process_metadata_pass
    OR baseline.handoff_pass::INTEGER IS DISTINCT FROM candidate.handoff_pass
    OR baseline.error_kind::VARCHAR IS DISTINCT FROM candidate.error_kind
    OR baseline.width::INTEGER IS DISTINCT FROM candidate.width
    OR baseline.height::INTEGER IS DISTINCT FROM candidate.height
    OR baseline.cfa::VARCHAR IS DISTINCT FROM candidate.cfa
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

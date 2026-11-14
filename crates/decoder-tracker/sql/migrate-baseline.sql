COPY (
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
        cfa::VARCHAR AS cfa,
        NULL::INTEGER AS compression_id,
        NULL::VARCHAR AS compression_name,
        NULL::VARCHAR AS storage,
        NULL::INTEGER AS bits_per_sample,
        NULL::VARCHAR AS sample_format,
        NULL::INTEGER AS predictor,
        NULL::INTEGER AS rows_per_strip,
        NULL::INTEGER AS tile_width,
        NULL::INTEGER AS tile_height
    FROM read_csv('@BASELINE@', header = true, all_varchar = true)
) TO '@OUTPUT@' (
    FORMAT CSV,
    HEADER,
    DELIMITER ',',
    QUOTE '"',
    NEW_LINE '\n'
);

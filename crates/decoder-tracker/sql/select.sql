COPY (
    SELECT
        path::VARCHAR AS path,
        size::BIGINT AS size,
        sha256::VARCHAR AS sha256
    FROM read_csv(
        '@MANIFEST@',
        header = true,
        all_varchar = true
    )
    WHERE path GLOB '@FILTER@'
    ORDER BY sha256, path
) TO '@OUTPUT@' (
    FORMAT CSV,
    HEADER,
    DELIMITER ',',
    QUOTE '"',
    NEW_LINE '\n'
);

SELECT count(*) AS selected
FROM read_csv('@OUTPUT@', header = true, all_varchar = true);

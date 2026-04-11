UPDATE books
SET status = CASE
    WHEN status IS NULL OR TRIM(status) = '' THEN 'unread'
    WHEN LOWER(TRIM(status)) = 'discovered' THEN 'unread'
    WHEN LOWER(TRIM(status)) = 'completed' THEN 'finished'
    WHEN LOWER(TRIM(status)) IN ('in_progress', 'in-progress') THEN 'reading'
    WHEN LOWER(TRIM(status)) IN ('unread', 'reading', 'finished') THEN LOWER(TRIM(status))
    ELSE 'unread'
END;

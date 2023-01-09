-- Optimize for Collections queries.
CREATE INDEX bcd_updates_lower_case_url_idx ON bcd_updates_read_table ((lower(mdn_url)));

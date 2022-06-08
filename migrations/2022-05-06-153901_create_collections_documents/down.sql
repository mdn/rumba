-- This file should undo anything in `up.sql`
DROP INDEX document_uri;
DROP INDEX idx_document_paths;
DROP TABLE collections;
DROP TABLE documents;
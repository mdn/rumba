CREATE TYPE notification_type AS ENUM ('content', 'compat');

CREATE TABLE notification_data 
(
    id           BIGSERIAL PRIMARY KEY,
    created_at   TIMESTAMP NOT NULL DEFAULT now(),
    updated_at   TIMESTAMP NOT NULL DEFAULT now(),
    text         TEXT      NOT NULL,
    url          TEXT      NOT NULL,
    data         JSONB,
    title        TEXT NOT NULL,
    type         notification_type NOT NULL,
    document_id  BIGSERIAL NOT NULL references documents(id)
);

CREATE TABLE notifications 
(    
    id BIGSERIAL PRIMARY KEY,
    user_id BIGSERIAL references users(id),
    starred boolean NOT NULL,
    read boolean NOT NULL,
    deleted_at TIMESTAMP,
    notification_data_id BIGSERIAL NOT NULL references notification_data (id)
);

CREATE TABLE watched_items
(    
    -- id BIGSERIAL PRIMARY KEY,
    user_id BIGSERIAL references users(id),
    document_id BIGSERIAL references documents(id),
    created_at TIMESTAMP NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, document_id)
);

CREATE INDEX notification_user_id on notifications(user_id); 
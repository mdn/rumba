CREATE TABLE user_subscription_transitions (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGSERIAL REFERENCES users (id) ON DELETE CASCADE,
    old_subscription_type subscription_type,
    new_subscription_type subscription_type,
    created_at TIMESTAMP NOT NULL DEFAULT now()
);

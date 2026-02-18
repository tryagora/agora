-- friends table stores directional friend relationships between matrix users
-- a friendship requires two rows: requester->addressee (pending) then both directions (accepted)
-- status: 'pending' | 'accepted' | 'blocked'
CREATE TABLE IF NOT EXISTS friends (
    id SERIAL PRIMARY KEY,
    requester_id VARCHAR(255) NOT NULL,   -- matrix user_id who sent the request
    addressee_id VARCHAR(255) NOT NULL,   -- matrix user_id who received it
    status VARCHAR(20) NOT NULL DEFAULT 'pending',
    -- dm_room_id caches the matrix room used for direct messages between these two users
    dm_room_id VARCHAR(255),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    CONSTRAINT friends_unique UNIQUE (requester_id, addressee_id),
    CONSTRAINT friends_status_check CHECK (status IN ('pending', 'accepted', 'blocked'))
);

CREATE INDEX IF NOT EXISTS idx_friends_requester ON friends(requester_id);
CREATE INDEX IF NOT EXISTS idx_friends_addressee ON friends(addressee_id);
CREATE INDEX IF NOT EXISTS idx_friends_status ON friends(requester_id, status);
